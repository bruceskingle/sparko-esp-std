use esp_idf_hal::{
    gpio::{PinDriver, Output},
    spi::SpiDeviceDriver,
};

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

use anyhow::Result;

use esp_idf_hal::{
    delay::Ets,
    peripherals::Peripherals,
    spi::{SpiDriverConfig, SpiConfig, Dma},
    units::Hertz,
};

use mipidsi::{
    interface::{Interface, InterfacePixelFormat},
    models::Model,
    
    Builder,
    models::ILI9341Rgb565,
    options::Orientation,
};

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Rectangle, PrimitiveStyle},
};

// mod display;
// use display::EspDi;

// pub struct MipiDsiDisplayManagerBuilder<'a, DI, MODEL, RST>
// where
//     DI: Interface,
//     MODEL: Model,
//     MODEL::ColorFormat: InterfacePixelFormat<DI::Word>,
// {

//     spi: SpiDeviceDriver<'a, esp_idf_hal::spi::SpiDriver<'a>>,
//     dc: PinDriver<'a, Output>,
//     // reset: PinDriver<'_, Output>,
//     backlight: PinDriver<'a, Output>,
//     builder: mipidsi::Builder<DI, MODEL, RST>,
// }

// impl<'a, DI, MODEL, RST> MipiDsiDisplayManagerBuilder<'a, DI, MODEL, RST>
// where
//     DI: Interface,
//     MODEL: Model,
//     MODEL::ColorFormat: InterfacePixelFormat<DI::Word>,
// {
//     fn new(
//         spi: SpiDeviceDriver<'a, esp_idf_hal::spi::SpiDriver<'a>>,
//         dc: PinDriver<'a, Output>,
//         // reset: PinDriver<'a, Output>,
//         backlight: PinDriver<'a, Output>,
//     ) -> MipiDsiDisplayManagerBuilder<'a> {
//         MipiDsiDisplayManagerBuilder {
//             spi,
//             dc,
//             backlight,
//         }
//     }


//     ///
//     /// Sets the invert color flag
//     ///
//     #[must_use]
//     pub fn invert_colors(mut self, color_inversion: ColorInversion) -> Self {
//         self.options.invert_colors = color_inversion;
//         self
//     }

//     ///
//     /// Sets the [ColorOrder]
//     ///
//     #[must_use]
//     pub fn color_order(mut self, color_order: ColorOrder) -> Self {
//         self.options.color_order = color_order;
//         self
//     }

//     ///
//     /// Sets the [Orientation]
//     ///
//     #[must_use]
//     pub fn orientation(mut self, orientation: Orientation) -> Self {
//         self.options.orientation = orientation;
//         self
//     }

//     ///
//     /// Sets refresh order
//     ///
//     #[must_use]
//     pub fn refresh_order(mut self, refresh_order: RefreshOrder) -> Self {
//         self.options.refresh_order = refresh_order;
//         self
//     }

//     /// Sets the display size.
//     ///
//     ///
//     #[must_use]
//     pub fn display_size(mut self, width: u16, height: u16) -> Self {
//         self.options.display_size = (width, height);
//         self
//     }

//     ///
//     /// Sets the display offset
//     ///
//     #[must_use]
//     pub fn display_offset(mut self, x: u16, y: u16) -> Self {
//         self.options.display_offset = (x, y);
//         self
//     }

//     pub fn build(self
//     ) -> anyhow::Result<MipiDsiDisplayManager<'a>> {
//         let di: EspDi<'a> = EspDi { spi: self.spi, dc: self.dc };

//         let mut delay = Ets;

//         let mut display: mipidsi::Display<EspDi<'a>, ILI9341Rgb565, mipidsi::NoResetPin> = match Builder::new(ILI9341Rgb565, di)
//             // .reset_pin(reset)
//             .display_size(240, 320)
//             .orientation(Orientation::new().flip_horizontal())
//             .init(&mut delay) {
//                 Ok(d) => d,
//                 Err(e) => anyhow::bail!("Display init error {:?}", e),
//             };

//         Ok(MipiDsiDisplayManager {
//             backlight: self.backlight,
//             display,
//         })
//     }
// }

pub struct MipiDsiDisplayManager<'a> {
    pub backlight: PinDriver<'a, Output>,
    pub display: mipidsi::Display<crate::display_mipidsi::EspDi<'a>, mipidsi::models::ILI9341Rgb565, mipidsi::NoResetPin>,
}

// impl<'a> MipiDsiDisplayManager<'a> {
//     pub fn builder(
//         spi: SpiDeviceDriver<'a, esp_idf_hal::spi::SpiDriver<'a>>,
//         dc: PinDriver<'a, Output>,
//         // reset: PinDriver<'_, Output>,
//         backlight: PinDriver<'_, Output>,
//     ) -> MipiDsiDisplayManager<'a> {
        
//         MipiDsiDisplayManagerBuilder::new(spi, dc, backlight)
//     }
// }


// pub fn start_display<'a>(
//     spi: SpiDeviceDriver<'a, esp_idf_hal::spi::SpiDriver<'a>>,
//     dc: PinDriver<'_, Output>,
//     // reset: PinDriver<'_, Output>,
//     mut backlight: PinDriver<'_, Output>,
// ) -> anyhow::Result<()> {
//     // esp_idf_svc::sys::link_patches();

//     // let peripherals = Peripherals::take().unwrap();
//     // let pins = peripherals.pins;

//     // // SPI
//     // let spi = SpiDeviceDriver::new_single(
//     //     peripherals.spi2,
//     //     pins.gpio14,
//     //     pins.gpio13,
//     //     Some(pins.gpio12),
//     //     Some(pins.gpio15),
//     //     &SpiDriverConfig::new().dma(Dma::Auto(4096)),
//     //     &SpiConfig::new().baudrate(Hertz(20_000_000)),
//     // )?;

//     // // GPIO
//     // let dc = PinDriver::output(pins.gpio2)?;
//     // let reset = PinDriver::output(pins.gpio4)?;
//     // let mut backlight = PinDriver::output(pins.gpio21)?;

//     // interface
//     let di = EspDi { spi, dc };

//     let mut delay = Ets;

//     let mut display = match Builder::new(ILI9341Rgb565, di)
//         // .reset_pin(reset)
//         .display_size(240, 320)
//         .orientation(Orientation::new().flip_horizontal())
//         .init(&mut delay) {
//             Ok(d) => d,
//             Err(e) => anyhow::bail!("Display init error {:?}", e),
//         };

//     // enable backlight
//     backlight.set_high()?;

//     // draw test
//     Rectangle::new(Point::new(0, 0), Size::new(240, 320))
//         .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
//         .draw(&mut display)?;

//     Ok(())
// }

// use anyhow::Result;

// use esp_idf_hal::{
//     delay::Ets,
//     gpio::PinDriver,
//     peripherals::Peripherals,
//     spi::{SpiDeviceDriver, SpiDriverConfig, SpiConfig, Dma},
//     units::Hertz,
// };

// use mipidsi::{
//     Builder,
//     models::ILI9341Rgb565,
//     options::Orientation,
// };

// use display_interface_spi::SPIInterface;

// struct Esp32DisplayInterface {
//     spi: SpiDeviceDriver<'static, SpiDriver<'static>>,
//     dc: PinDriver<'static, gpio::Output>,
// }

// impl mipidsi::interface::Interface for Esp32DisplayInterface {
//     type Error = esp_idf_hal::spi::SpiError;
//     type Word = u8;

//     fn send_command(
//         &mut self,
//         cmd: u8,
//         args: &[u8],
//     ) -> Result<(), Self::Error> {
//         self.dc.set_low().ok();
//         self.spi.write(&[cmd])?;

//         if !args.is_empty() {
//             self.dc.set_high().ok();
//             self.spi.write(args)?;
//         }

//         Ok(())
//     }

//     fn send_pixels<const N: usize>(
//         &mut self,
//         data: impl IntoIterator<Item = [u8; N]>,
//     ) -> Result<(), Self::Error> {
//         self.dc.set_high().ok();

//         for chunk in data {
//             self.spi.write(&chunk)?;
//         }

//         Ok(())
//     }

//     fn send_repeated_pixel<const N: usize>(
//         &mut self,
//         pixel: [u8; N],
//         count: u32,
//     ) -> Result<(), Self::Error> {
//         self.dc.set_high().ok();

//         let mut buf = [0u8; 64];
//         let mut remaining = count;

//         while remaining > 0 {
//             let n = core::cmp::min(remaining, (buf.len() / N) as u32);
//             let mut i = 0;

//             for _ in 0..n {
//                 for b in pixel {
//                     buf[i] = b;
//                     i += 1;
//                 }
//             }

//             self.spi.write(&buf[..i])?;
//             remaining -= n;
//         }

//         Ok(())
//     }
// }

// fn foo() {
//     let spi = SpiDeviceDriver::new_single(
//         peripherals.spi2,
//         sck,
//         mosi,
//         Some(miso),
//         Some(cs),
//         &SpiDriverConfig::new(),
//         &SpiConfig::new().baudrate(Hertz(20_000_000)),
//     )?;

//     let di = Esp32DisplayInterface { spi, dc };

//     let mut delay = Ets;

//     let display = Builder::new(ILI9341Rgb565, di)
//         .display_size(240, 320)
//         .reset_pin(reset)
//         .orientation(Orientation::new().flip_horizontal())
//         .init(&mut delay)?;
// }

// pub struct Cyd<'d> {
//     pub display: mipidsi::Display<
//         SPIInterface<
//             SpiDeviceDriver<'d, esp_idf_hal::spi::SpiDriver<'d>>,
//             PinDriver<'d, esp_idf_hal::gpio::Output>,
//         >,
//         ILI9341Rgb565,
//         mipidsi::NoResetPin,
//     >,
//     pub backlight: PinDriver<'d, esp_idf_hal::gpio::Output>,
// }

// pub fn init() -> Result<Cyd<'static>> {
//     let peripherals = Peripherals::take().unwrap();
//     let pins = peripherals.pins;

//     // ---------------- SPI DRIVER ----------------
//     let spi = SpiDeviceDriver::new_single(
//         peripherals.spi2,
//         pins.gpio14, // SCLK
//         pins.gpio13, // MOSI
//         Some(pins.gpio12), // MISO
//         Some(pins.gpio15), // CS (driver-managed)
//         &SpiDriverConfig::new().dma(Dma::Auto(4096)),
//         &SpiConfig::new().baudrate(Hertz(20_000_000)),
//     )?;

//     // ---------------- GPIO ----------------
//     let dc = PinDriver::output(pins.gpio2)?;
//     let reset = PinDriver::output(pins.gpio4)?; // IMPORTANT: real reset pin
//     let backlight = PinDriver::output(pins.gpio21)?;

//     // ---------------- DISPLAY INTERFACE ----------------
//     let di = SPIInterface::new(spi, dc);

//     let mut delay = Ets;

//     let display = Builder::new(ILI9341Rgb565, di)
//         .reset_pin(reset)
//         .display_size(240, 320)
//         .orientation(Orientation::new().flip_horizontal())
//         .init(&mut delay)?;

//     Ok(Cyd {
//         display,
//         backlight,
//     })
// }

// // use anyhow::Result;
// // use esp_idf_hal::{
// //     delay::Ets,
// //     gpio::{PinDriver, Output},
// //     peripherals::Peripherals,
// //     spi::{config::Config as SpiConfig, SpiDeviceDriver, SpiDriver, SpiDriverConfig, Dma}
// // };
// // use mipidsi::{
// //     Builder as DisplayBuilder,
// //     models::ILI9341Rgb565,
// //     options::Orientation,
// // };
// // use display_interface_spi::SPIInterface;
// // use esp_idf_hal::units::Hertz;


// // type SpiDev = SpiDeviceDriver<'static, SpiDriver<'static>>;
// // type DcPin = PinDriver<'static, Output>;

// // type DisplayType = mipidsi::Display<
// //     SPIInterfaceNoCS<SpiDev, DcPin>,
// //     ILI9341Rgb565,
// //     mipidsi::NoResetPin,
// // >;

// // pub struct Cyd {
// //     pub display: DisplayType,
// //     pub backlight_pin: DcPin,
// //     pub led_red_pin: DcPin,
// //     pub led_green_pin: DcPin,
// //     pub led_blue_pin: DcPin,
// // }

// // // pub struct Cyd {
// // //     pub display: mipidsi::Display<
// // //         SPIInterface<SpiDeviceDriver<'static>, PinDriver<'static, Output>>,
// // //         ILI9341Rgb565,
// // //         mipidsi::NoResetPin,
// // //     >,
// // //     pub backlight_pin: PinDriver<'static, Output>,
// // //     pub led_red_pin: PinDriver<'static, Output>,
// // //     pub led_green_pin: PinDriver<'static, Output>,
// // //     pub led_blue_pin: PinDriver<'static, Output>,
// // // }

// // pub struct Builder {

// // }
// // impl Builder {
// //     pub fn init(self) -> Result<Cyd> {
// //         let peripherals = Peripherals::take().unwrap();

// //         let pins = peripherals.pins;

// //         // === SPI pins ===
// //         let sclk = pins.gpio14;
// //         let mosi = pins.gpio13;
// //         let miso = pins.gpio12;
// //         let cs   = pins.gpio15;
// //         let dc   = pins.gpio2;
// //         let bl   = pins.gpio21;

// //         // === SPI config ===
// //         let spi_config = SpiConfig::new().baudrate(Hertz(20_000_000));

// //         let driver_config = SpiDriverConfig::new().dma(Dma::Auto(4096));

// //         // === SPI device ===
// //         let spi = SpiDeviceDriver::new_single(
// //             peripherals.spi2,
// //             sclk,
// //             mosi,
// //             Some(miso),
// //             Some(cs),
// //             &driver_config,
// //             &spi_config,
// //         )?;

// //         // === GPIO ===
// //         let dc = PinDriver::output(dc)?;
// //         let backlight = PinDriver::output(bl)?;

// //         let led_red   = PinDriver::output(pins.gpio4)?;
// //         let led_green = PinDriver::output(pins.gpio16)?;
// //         let led_blue  = PinDriver::output(pins.gpio17)?;

// //         // === Display interface (v0.5 API) ===
// //         let di = SPIInterfaceNoCS::new(spi, dc);

// //         // === Delay ===
// //         let mut delay = Ets;

// //         // === Display ===
// //         let display = DisplayBuilder::new(ILI9341Rgb565, di)
// //             .display_size(240, 320)
// //             .orientation(
// //                 self.orientation
// //                     .unwrap_or_else(|| Orientation::new().flip_horizontal()),
// //             )
// //             .init(&mut delay)?;

// //         Ok(Cyd {
// //             display,
// //             backlight_pin: backlight,
// //             led_red_pin: led_red,
// //             led_green_pin: led_green,
// //             led_blue_pin: led_blue,
// //         })
// //     }
// // }