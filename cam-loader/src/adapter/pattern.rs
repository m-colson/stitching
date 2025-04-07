use zerocopy::FromBytes;

use crate::{
    Result,
    loader::{Loader, OwnedWriteBuffer},
};

use super::Config;

pub fn from_spec<B: OwnedWriteBuffer + Send + 'static>(
    spec: &Config,
    color: [u8; 3],
    grid_size: u32,
) -> Result<Loader<B>> {
    const CHANS: u32 = 4;
    const OFF_COLOR: u32 = u32::from_le_bytes([0x18, 0x18, 0x17, 255]);

    let [w, h] = spec.resolution;

    if w % grid_size != 0 {
        return Err(crate::Error::Other(format!(
            "invalid grid_size {grid_size} for resolution {w}*{h}. width must be a multiple of grid_size"
        )));
    }

    Ok(Loader::new_blocking(w, h, CHANS, move |dest| {
        let dest = <[u32]>::mut_from_bytes(dest).unwrap();

        // NOTE: this is written in a non-obvious way to massively improve performace
        let mut off = 0;
        let mut render_row = move |mut sec_on: bool| {
            for _ in (0..w).step_by(grid_size as _) {
                let pixel = if sec_on {
                    u32::from_le_bytes([color[0], color[1], color[2], 255])
                } else {
                    OFF_COLOR
                };
                dest[off..][..grid_size as usize].fill(pixel);

                off += grid_size as usize;
                sec_on = !sec_on;
            }
        };

        let on_off_size = grid_size * 2;
        let num_v_on_offs = h / on_off_size;
        let last_v_on_off_size = h % on_off_size;
        for _ in 0..num_v_on_offs {
            for _ in 0..grid_size {
                render_row(false);
            }
            for _ in 0..grid_size {
                render_row(true);
            }
        }

        for _ in 0..(grid_size.min(last_v_on_off_size)) {
            render_row(false);
        }
        for _ in 0..(grid_size.min(last_v_on_off_size.saturating_sub(16))) {
            render_row(true);
        }
    }))
}
