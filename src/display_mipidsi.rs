use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Rectangle, PrimitiveStyle},
};
use esp_idf_hal::{
    gpio::{PinDriver, Output},
    spi::SpiDeviceDriver,
};
use sparko_embedded_std::{Color, DisplayManager, InitStatus, Status};

use crate::to_rgb565;


pub struct EspDi<'d> {
    pub spi: SpiDeviceDriver<'d, esp_idf_hal::spi::SpiDriver<'d>>,
    pub dc: PinDriver<'d, Output>,
}

impl<'d> mipidsi::interface::Interface for EspDi<'d> {

    type Word = u8;
    type Error = esp_idf_hal::spi::SpiError;
    const KIND: mipidsi::interface::InterfaceKind = mipidsi::interface::InterfaceKind::Serial4Line;

    fn send_command(&mut self, cmd: u8, args: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low().ok();
        self.spi.write(&[cmd])?;

        if !args.is_empty() {
            self.dc.set_high().ok();
            self.spi.write(args)?;
        }

        Ok(())
    }

    fn send_pixels<const N: usize>(
        &mut self,
        pixels: impl IntoIterator<Item = [u8; N]>,
    ) -> Result<(), Self::Error> {
        self.dc.set_high().ok();

        for chunk in pixels {
            self.spi.write(&chunk)?;
        }

        Ok(())
    }

    fn send_repeated_pixel<const N: usize>(
        &mut self,
        pixel: [u8; N],
        mut count: u32,
    ) -> Result<(), Self::Error> {
        self.dc.set_high().ok();

        let mut buf = [0u8; 64];

        while count > 0 {
            let n = core::cmp::min(count, (buf.len() / N) as u32);
            let mut idx = 0;

            for _ in 0..n {
                for b in pixel {
                    buf[idx] = b;
                    idx += 1;
                }
            }

            self.spi.write(&buf[..idx])?;
            count -= n;
        }

        Ok(())
    }
}



pub struct MipiDsiDisplayManager<'a> {
    pub backlight: PinDriver<'a, Output>,
    pub display: mipidsi::Display<crate::display_mipidsi::EspDi<'a>, mipidsi::models::ILI9341Rgb565, mipidsi::NoResetPin>,
    pub size: Size,
}

impl MipiDsiDisplayManager<'_> {
}

impl DisplayManager for MipiDsiDisplayManager<'_> {
    fn fill_color(&mut self, color: Color) -> anyhow::Result<()> {
        Rectangle::new(Point::new(0, 0), self.size)
            .into_styled(PrimitiveStyle::with_fill(to_rgb565(color)))
            .draw(&mut self.display)?;
        Ok(())
    }

    
}
