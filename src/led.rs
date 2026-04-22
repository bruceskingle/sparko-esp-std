use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
use esp_idf_hal::gpio::OutputPin;
use esp_idf_hal::ledc::{LedcTimer, LowSpeed, LedcChannel};
use log::info;

use esp_idf_hal::{gpio::PinDriver, ledc::LedcDriver};
use esp_idf_hal::units::*;
use sparko_embedded_std::Status;



pub trait LedManager {
    fn set_on(&self) -> anyhow::Result<()>;
    fn set_off(&self) -> anyhow::Result<()>;
    fn set_status(&self, status: &Status) -> anyhow::Result<()>;
    // fn set_led_initializing(&self) -> anyhow::Result<()>;
    // fn set_led_running(&self) -> anyhow::Result<()>;
    // fn set_led_admin(&self) -> anyhow::Result<()>;
    // fn set_led_error(&self) -> anyhow::Result<()>;
}

pub struct RgbLedManager<'a> {
    inverted: bool,
    brightness: u8,
    led_timer_driver: esp_idf_hal::ledc::LedcTimerDriver<'a, esp_idf_hal::ledc::LowSpeed>,
    led_channel_red: Arc<Mutex<esp_idf_hal::ledc::LedcDriver<'a>>>,
    led_channel_green: Arc<Mutex<esp_idf_hal::ledc::LedcDriver<'a>>>,
    led_channel_blue: Arc<Mutex<esp_idf_hal::ledc::LedcDriver<'a>>>,
}

impl<'a> RgbLedManager<'a> {
    pub fn new<T: LedcTimer<SpeedMode = LowSpeed> + 'a, 
        CR: LedcChannel<SpeedMode = LowSpeed> + 'a, PR: OutputPin + 'a,
        CG: LedcChannel<SpeedMode = LowSpeed> + 'a, PG: OutputPin + 'a,
        CB: LedcChannel<SpeedMode = LowSpeed> + 'a, PB: OutputPin + 'a,
        >(
        inverted: bool,
        brightness: u8,
        timer0: T,
        red_channel: CR,
        red_pin: PR,
        green_channel: CG,
        green_pin: PG,
        blue_channel: CB,
        blue_pin: PB,
    ) -> anyhow::Result<Self> {
        let led_timer_driver = esp_idf_hal::ledc::LedcTimerDriver::new(timer0,
            &esp_idf_hal::ledc::config::TimerConfig::new().frequency(1000.Hz()))?;
    
        let led_channel_red = Arc::new(Mutex::new(LedcDriver::new(red_channel, &led_timer_driver, red_pin)?));
        let led_channel_green = Arc::new(Mutex::new(LedcDriver::new(green_channel, &led_timer_driver, green_pin)?));
        let led_channel_blue = Arc::new(Mutex::new(LedcDriver::new(blue_channel, &led_timer_driver, blue_pin)?));

        Ok(Self {
            inverted,
            brightness,
            led_timer_driver,
            led_channel_red,
            led_channel_green,
            led_channel_blue,
         })
    }

    fn apply_inversion(&self, value: u8) -> u8 {
        let value = (value as u16 * self.brightness as u16 / 255) as u8; 
        if self.inverted {
            255 - value
        } else {
            value
        }
    }

    pub fn set_color(&self, r: u8, g: u8, b: u8) -> anyhow::Result<()> {
        info!("Set led color r={} g={} b={}", r, g, b);
        self.led_channel_red.lock().unwrap().set_duty(self.apply_inversion(r) as u32)?;
        self.led_channel_green.lock().unwrap().set_duty(self.apply_inversion(g) as u32)?;
        self.led_channel_blue.lock().unwrap().set_duty(self.apply_inversion(b) as u32)?;
        Ok(())
    }
}

impl LedManager for RgbLedManager<'_> {
    fn set_on(&self) -> anyhow::Result<()> {
        self.set_color(255, 255, 255)?;
        Ok(())
    }

    fn set_off(&self)  -> anyhow::Result<()>{
        self.set_color(0, 0, 0)?;
        Ok(())
    }

    // fn set_led_initializing(&self) -> anyhow::Result<()> {
    //     self.set_color(255, 255, 0)?;
    //     Ok(())
    // }
    
    // fn set_led_running(&self) -> anyhow::Result<()> {
    //     self.set_color(0, 255, 0)?;
    //     Ok(())
    // }
    
    // fn set_led_admin(&self) -> anyhow::Result<()> {
    //     self.set_color(0, 0, 255)?;
    //     Ok(())
    // }
    
    // fn set_led_error(&self) -> anyhow::Result<()> {
    //     self.set_color(255, 0, 0)?;
    //     Ok(())
    // }
    
    fn set_status(&self, status: &Status) -> anyhow::Result<()> {
        match status {
            Status::Initializing(_) => self.set_color(255, 255, 0)?,
            Status::Running => self.set_color(0, 255, 0)?,
            Status::Setup => self.set_color(0, 0, 255)?,
            Status::Error => self.set_color(255, 0, 0)?,
        };
        Ok(())
    }
}

struct InvertiblePinDriver<'a> {
    pin_driver: PinDriver<'a, esp_idf_hal::gpio::Output>,
    inverted: bool,
}

impl<'a> InvertiblePinDriver<'a> {
    fn new<P: OutputPin + 'a>(pin: P, inverted: bool) -> Self {
        let pin_driver: PinDriver<'a, esp_idf_hal::gpio::Output> = PinDriver::output(pin).unwrap();
        Self {
            pin_driver,
            inverted,
        }
    }

    fn on(&mut self) -> anyhow::Result<()> {
        if self.inverted {
            self.pin_driver.set_low()?;
        } else {
            self.pin_driver.set_high()?;
        }
        Ok(())
    }

    fn off(&mut self) -> anyhow::Result<()> {
        if self.inverted {
            self.pin_driver.set_high()?;
        } else {
            self.pin_driver.set_low()?;
        }
        Ok(())
    }
    
}

// struct WaitNotifyGuard<'a> {
//     lock: std::sync::MutexGuard<'a, FlashConfig>,
//     cvar: &'a Condvar,
// }

#[derive(Clone)]
struct FlashConfig {
    flashes: u32,
    burst: Duration,
    pause: Duration,
}

struct Shared {
    config: FlashConfig,
    updated: bool,
}

pub struct MonoLedManager {
    shared_state: Arc<(Mutex<Shared>, Condvar)>,
}

impl MonoLedManager {
    pub fn new<P: OutputPin + 'static>(
        inverted: bool,
        pin: P,
    ) -> anyhow::Result<Self> {
        let shared_state = Arc::new((Mutex::new(
            Shared {
                config: FlashConfig {
                    flashes: 3,
                    burst: Duration::from_secs(1),
                    pause: Duration::from_secs(1),
                },
                updated: false
            }), Condvar::new()));

        let shared_state_clone = shared_state.clone();

        let result = Self {
            shared_state,
         };

        thread::spawn(move || {
            let mut pin_driver = InvertiblePinDriver::new(pin, inverted);

            let (lock , cond_var) = &*shared_state_clone;

            loop {

                let mut state = lock.lock().unwrap();
                let config = state.config.clone();
                state.updated = false;
                drop(state);

                // info!("Flashing led: flashes={}, burst={:?}, pause={:?}", config.flashes, config.burst, config.pause);
                
                if config.flashes == 0 {
                    Self::wait_or_interrupt(lock, cond_var, Duration::from_secs(300));
                }
                else if config.flashes == 1 && config.burst == Duration::from_secs(0) && config.pause == Duration::from_secs(0) {
                    pin_driver.on().unwrap();
                    Self::wait_or_interrupt(lock, cond_var, Duration::from_secs(300));
                }
                else {
                    Self::flash(&mut pin_driver, lock, cond_var, config);
                }
            }
        });
        
        Ok(result)
    }

    fn flash(pin_driver: &mut InvertiblePinDriver, lock: &Mutex<Shared>, cond_var: &Condvar, config: FlashConfig) {
        let on_off = config.burst / (config.flashes * 2);

        for _ in 0..config.flashes {
            pin_driver.on().unwrap();
            if Self::wait_or_interrupt(lock, cond_var, on_off) {
                return;
            }

            pin_driver.off().unwrap();
            if Self::wait_or_interrupt(lock, cond_var, on_off) {
                return;
            }
        }
        
        Self::wait_or_interrupt(lock, cond_var, config.pause);
    }

    fn wait_or_interrupt(
        lock: &Mutex<Shared>,
        cvar: &Condvar,
        timeout: Duration,
    ) -> bool {
        let state = lock.lock().unwrap();

        let (state, _) = cvar
            .wait_timeout(state, timeout)
            .unwrap();

        state.updated
    }

    pub fn set_flash_config(&self, flashes: u32, burst: Duration, pause: Duration) -> anyhow::Result<()> {
        if burst < Duration::from_millis(100) || pause < Duration::from_millis(100) {
            anyhow::bail!("Burst and pause durations must be at least 100ms");
        }

        let (lock , cond_var) = &*self.shared_state;
        let mut shared_state = lock.lock().unwrap();
        
        shared_state.config.flashes = flashes;
        shared_state.config.burst = burst;
        shared_state.config.pause = pause;
        
        shared_state.updated = true;

        cond_var.notify_all();
        Ok(())
    }

    pub fn set_flashes(&self, flashes: u32) {
        let (lock , cond_var) = &*self.shared_state;
        let mut shared_state = lock.lock().unwrap();

        shared_state.config.flashes = flashes;
        
        shared_state.updated = true;

        cond_var.notify_all();
    }
}

impl LedManager for MonoLedManager {
    
    fn set_on(&self) -> anyhow::Result<()> {
        info!("Set led on");
        
        let (lock , cond_var) = &*self.shared_state;
        let mut shared_state = lock.lock().unwrap();

        shared_state.config.flashes = 1;
        shared_state.config.burst = Duration::from_secs(0);
        shared_state.config.pause = Duration::from_secs(0);

        shared_state.updated = true;

        cond_var.notify_all();
        Ok(())
    }

    fn set_off(&self) -> anyhow::Result<()> {
        info!("Set led off");
        
        let (lock , cond_var) = &*self.shared_state;
        let mut shared_state = lock.lock().unwrap();

        shared_state.config.flashes = 0;

        shared_state.updated = true;

        cond_var.notify_all();
        Ok(())
    }

    fn set_status(&self, status: &Status) -> anyhow::Result<()> {
        match status {
            Status::Initializing(_) => self.set_flashes(1),
            Status::Running => self.set_off()?,
            Status::Setup => self.set_flashes(2),
            Status::Error => self.set_flashes(3),
        };
        Ok(())
    }

    // fn set_led_initializing(&self) -> anyhow::Result<()> {
    //     self.set_flashes(4);
    //     // self.set_off();
    //     Ok(())
    // }
    
    // fn set_led_running(&self) -> anyhow::Result<()> {
    //     // self.set_flashes(1);
    //     self.set_off();
    //     Ok(())
    // }
    
    // fn set_led_admin(&self) -> anyhow::Result<()> {
    //     self.set_flashes(2);
    //     Ok(())
    // }
    
    // fn set_led_error(&self) -> anyhow::Result<()> {
    //     self.set_flashes(5);
    //     Ok(())
    // }
}


pub struct SimpleLedManager<'d> {
    inverted: bool,
    led: Arc<Mutex<PinDriver<'d, esp_idf_hal::gpio::Output>>>,
}

impl<'d> SimpleLedManager<'d> {
    pub fn new<T: OutputPin + 'd>(inverted: bool, pin: T) -> Self {
        let led = Arc::new(Mutex::new(PinDriver::output(pin).unwrap()));
        Self { inverted, led }
    }
}

impl LedManager for SimpleLedManager<'_> {
    fn set_on(&self) -> anyhow::Result<()> {
        info!("Set led on");
        if self.inverted {
            self.led.lock().unwrap().set_low()?;
        } else {
            self.led.lock().unwrap().set_high()?;
        }
        Ok(())
    }

    fn set_off(&self) -> anyhow::Result<()> {
        info!("Set led off");
        if self.inverted {
            self.led.lock().unwrap().set_high()?;
        } else {
            self.led.lock().unwrap().set_low()?;
        }
        Ok(())
    }

    fn set_status(&self, status: &Status) -> anyhow::Result<()> {
        match status {
            Status::Initializing(_) => self.set_on()?,
            Status::Running => self.set_off()?,
            Status::Setup => self.set_on()?,
            Status::Error => self.set_on()?,
        };
        Ok(())
    }
    
    // fn set_led_initializing(&self) -> anyhow::Result<()> {
    //     self.set_on();
    //     Ok(())
    // }
    
    // fn set_led_running(&self) -> anyhow::Result<()> {
    //     self.set_off();
    //     Ok(())
    // }
    
    // fn set_led_admin(&self) -> anyhow::Result<()> {
    //     self.set_off();
    //     Ok(())
    // }
    
    // fn set_led_error(&self) -> anyhow::Result<()> {
    //     self.set_on();
    //     Ok(())
    // }

}