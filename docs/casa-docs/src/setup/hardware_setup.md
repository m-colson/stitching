# Hardware/Operating System Setup

## Flashing

The J4011 comes pre-flashed with what appears to be a very old JetPack, so we need to update it.

Seeed studio provides [instructions here](https://wiki.seeedstudio.com/reComputer_J4012_Flash_Jetpack/) to install *JetPack 6.1*.

There are a few pitfalls to watch out for:
- A computer with Ubuntu 20.04 or greater is *required* to host the flashing process.
- It may be necessary to flash the board with JetPack 5.1.3 *before* flashing it again with a newer version. Once booted into the 5.1.3 OS, wait for some time so the board's firmware can be updated.

## First Time OS Setup
When prompted with the Ubuntu user creation screen, make sure the main account 
username is `casa`, as the following steps expect it.

## System Configuration

### Install packages

```sh
sudo apt update
sudo apt upgrade
sudo apt install clang hostapd
```

### Add necessary files to system
- Camera Overrides - [download](../files/camera_overrides.isp) ([here are some for this and other cameras](https://docs.arducam.com/Nvidia-Jetson-Camera/Application-note/Fix-Red-Tint-with-ISP-Tuning/#software-setting))
```sh
sudo cp camera_overrides.isp /var/nvidia/nvcam/settings/
sudo chmod 664 /var/nvidia/nvcam/settings/camera_overrides.isp
sudo chown root:root /var/nvidia/nvcam/settings/camera_overrides.isp
```
- Seeed board CSI camera configuration overlay - [download](../files/tegra234-p3767-camera-p3768-imx219-dual-seeed.dtbo)
```sh
sudo cp tegra234-p3767-camera-p3768-imx219-dual-seeed.dtbo /boot
```
- Add boot entry with overlay - [download](../files/extlinux.conf)
```sh
sudo cat extlinux.conf >> /boot/extlinux/extlinux.conf
sudo nano /boot/extlinux/extlinux.conf
# Change the default on the first line to SeeedIMX219Dual and save
```
- Add Access Point configuration - [download](../files/hostapd.conf)
```sh
sudo cp hostapd.conf /etc/hostapd/hostapd.conf
sudo nano /etc/hostapd/hostapd.conf
# Change <<WIFI_PASSWORD>> to the wifi password and save
```
- Add systemd service - [download](../files/casa.service)
```sh
sudo cp casa.service /etc/systemd/system/casa.service
```

### Change Power Mode to 20w
This can be done by signing into the system through a plugged in display and changing the tile on the top right of the screen from 15w to 20w; then reboot.
