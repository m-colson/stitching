package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"path"
	"path/filepath"
	"regexp"
	"strings"

	"al.essio.dev/pkg/shellescape"
	"github.com/pkg/sftp"
	"golang.org/x/crypto/ssh"
)

func main() {
	includes := flag.String("i", "", "comma seperated list of globbed files to copy")
	pkgName := flag.String("n", "stitch", "name to use within destination temp folder")
	binName := flag.String("bin", "", "name of executable in destination")
	runPerf := flag.Bool("perf", false, "whether to run the file or flamegraph")
	flag.Parse()

	execPath := flag.Arg(0)
	runArgs := flag.Args()[1:]

	cfg := Config{}.WithEnv()

	conn, err := cfg.DialSSH()
	if err != nil {
		log.Fatal("unable to connect: ", err)
	}
	defer conn.Close()

	dstFolder := path.Join("/home", cfg.Username, ".upload-runner-temp", *pkgName)

	dstFilename := *binName
	if dstFilename == "" {
		dstFilename = path.Base(execPath)
	}
	dstFilename = path.Join(dstFolder, dstFilename)

	err = conn.SendFile(execPath, dstFilename, 0777)
	if err != nil {
		log.Fatal("unable to transfer executable: ", err)
	}

	for _, inc := range strings.Split(*includes, ",") {
		matches, err := filepath.Glob(inc)
		if err != nil {
			log.Fatal("bad glob: ", err)
		}
		for _, p := range matches {
			log.Printf("sending %q", p)
			err = conn.SendFile(p, path.Join(dstFolder, filepath.ToSlash(p)), 0666)
			if err != nil {
				log.Fatalf("unable to transfer included file %q: %s", inc, err)
			}
		}
	}

	session, err := conn.SpawnPty()
	if err != nil {
		log.Fatal("unable to create session: ", err)
	}
	defer session.Close()

	session.Stdin = os.Stdin
	session.Stdout = os.Stdout
	session.Stderr = os.Stderr

	if *runPerf {
		cmd := formatRunCmd(dstFolder, "/home/mc/.cargo/bin/flamegraph", append([]string{"-v", "--", dstFilename}, runArgs...))

		if err := session.Run(cmd); err != nil {
			log.Fatal("failed to run: ", err)
		}

		if err := conn.RecvFile(path.Join(dstFolder, "flamegraph.svg"), "flamegraph.svg"); err != nil {
			log.Fatal("failed to receive flamegraph: ", err)
		}
	} else {
		if err := session.Run(formatRunCmd(dstFolder, dstFilename, runArgs)); err != nil {
			log.Fatal("failed to run: ", err)
		}
	}
}

func formatRunCmd(dir string, prog string, args []string) string {
	return strings.Join([]string{"cd", shellescape.Quote(dir), ";", prog, shellescape.QuoteCommand(args)}, " ")
}

type Config struct {
	Username    string
	PrivKeyPath string
	Password    string
	Host        string
	Port        string
}

var uploaderRe = regexp.MustCompile(`(.*?)(?:\[(.*)\])?(?::(.*))?\@([^\s:]*)(:\d+)?`)

func (inp Config) WithEnv() (out Config) {
	out = inp

	if uploader, ok := os.LookupEnv("UPLOADER"); ok {
		subs := uploaderRe.FindStringSubmatch(uploader)
		if subs == nil {
			panic(fmt.Errorf("environment variable UPLOADER is not of the form \"user([key-path]|:pass)@host(:port)\""))
		}
		out.Username = subs[1]
		out.PrivKeyPath = subs[2]
		out.Password = subs[3]
		out.Host = subs[4]
		out.Port = subs[5]
	}

	if username, ok := os.LookupEnv("UPLOAD_USER"); ok {
		out.Username = username
	}

	if privkey, ok := os.LookupEnv("UPLOAD_PRIVKEY"); ok {
		out.PrivKeyPath = privkey
	}

	if password, ok := os.LookupEnv("UPLOAD_PASS"); ok {
		out.Password = password
	}

	if host, ok := os.LookupEnv("UPLOAD_HOST"); ok {
		out.Host = host
	}

	if port, ok := os.LookupEnv("UPLOAD_PORT"); ok {
		out.Port = port
	}

	return
}

func (cfg Config) DialSSH() (ClientW, error) {
	Auth := []ssh.AuthMethod{}
	if cfg.PrivKeyPath != "" {
		keyData, err := os.ReadFile(cfg.PrivKeyPath)
		if err != nil {
			return ClientW{}, err
		}

		var signer ssh.Signer
		if cfg.Password == "" {
			signer, err = ssh.ParsePrivateKey(keyData)
		} else {
			signer, err = ssh.ParsePrivateKeyWithPassphrase(keyData, []byte(cfg.Password))
		}
		if err != nil {
			return ClientW{}, err
		}

		Auth = append(Auth, ssh.PublicKeys(signer))
	} else if cfg.Password != "" {
		Auth = append(Auth, ssh.Password(cfg.Password))
	}

	config := &ssh.ClientConfig{
		User: cfg.Username,
		Auth: Auth,
		// TODO: FIX THIS
		HostKeyCallback: ssh.InsecureIgnoreHostKey(),
	}

	if cfg.Host == "" {
		return ClientW{}, fmt.Errorf("can't use Config.DialSSH() with an empty hostname")
	}
	if cfg.Port == "" {
		cfg.Port = ":22"
	}

	conn, err := ssh.Dial("tcp", cfg.Host+cfg.Port, config)
	return ClientW{conn}, err
}

type ClientW struct {
	*ssh.Client
}

func (c ClientW) SendFile(src string, dst string, mode os.FileMode) error {
	ftp, err := sftp.NewClient(c.Client, sftp.UseConcurrentWrites(false))
	if err != nil {
		return err
	}
	defer ftp.Close()

	src_file, err := os.Open(src)
	if err != nil {
		return err
	}
	defer src_file.Close()

	dstDir, _ := path.Split(dst)
	if err = ftp.MkdirAll(dstDir); err != nil {
		log.Println("dir failed", dstDir)
		return err
	}

	dst_file, err := ftp.Create(dst)
	if err != nil {
		return err
	}
	defer dst_file.Close()

	_, err = dst_file.ReadFrom(src_file)
	if err != nil {
		return err
	}

	return dst_file.Chmod(mode)
}

func (c ClientW) RecvFile(src string, dst string) error {
	ftp, err := sftp.NewClient(c.Client, sftp.UseConcurrentWrites(false))
	if err != nil {
		return err
	}
	defer ftp.Close()

	srcFile, err := ftp.Open(src)
	if err != nil {
		return err
	}
	defer srcFile.Close()

	dstFile, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer dstFile.Close()

	_, err = dstFile.ReadFrom(srcFile)
	if err != nil {
		return err
	}

	return nil
}

func (c ClientW) SpawnPty() (sw SessionW, err error) {
	session, err := c.Client.NewSession()
	if err != nil {
		return
	}

	modes := ssh.TerminalModes{
		ssh.ECHO:          0,     // disable echoing
		ssh.TTY_OP_ISPEED: 14400, // input speed = 14.4kbaud
		ssh.TTY_OP_OSPEED: 14400, // output speed = 14.4kbaud
	}

	err = session.RequestPty("xterm", 40, 80, modes)
	return SessionW{session}, err
}

type SessionW struct {
	*ssh.Session
}
