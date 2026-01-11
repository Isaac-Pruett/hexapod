use crate::Irqs;
use core::time::Duration;
use embassy_rp::Peri;
use embassy_rp::Peripherals;
use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::pio::{Instance, Pio};
use embassy_rp::pio_programs::pwm::{PioPwm, PioPwmProgram};
use embassy_rp::pwm::{Config as PwmConfig, Pwm, SetDutyCycle};
use embassy_time::Timer;

const STD_MAX_US: u64 = 870;
const STD_MIN_US: u64 = 190;
const MAX_SPEED: f32 = 60.0 / 150.0; // DEG/MILLISEC

/// Simple Servo controller using PIO PWM
pub struct ServoFromPIO<'d, T: Instance, const SM: usize> {
    pwm: PioPwm<'d, T, SM>,
    /// Pulse width for 0° (in microseconds)
    min_us: u64,
    /// Pulse width for max_angle (in microseconds)
    max_us: u64,
    /// Maximum angle the servo supports (usually 180.0 or 270.0)
    max_angle: f32,
    /// Current servo angle
    pub angle: f32,
}

impl<'d, T: Instance, const SM: usize> ServoFromPIO<'d, T, SM> {
    /// Create a new servo controller
    pub fn new(
        pwm: PioPwm<'d, T, SM>,
        min_us: u64,
        max_us: u64,
        max_angle: f32,
        angle: f32,
    ) -> Self {
        Self {
            pwm,
            min_us,
            max_us,
            max_angle,
            angle,
        }
    }

    /// Most common convenience constructor - 180° servo with standard timings
    pub fn new_standard_180(pwm: PioPwm<'d, T, SM>) -> Self {
        Self::new(pwm, STD_MIN_US, STD_MAX_US, 180.0, 0.0)
    }

    pub fn start(&mut self) {
        self.pwm.set_period(Duration::from_millis(20));
        self.pwm.start();
    }

    /// Write angle in degrees (0.0 ..= max_angle)
    pub fn write(&mut self, angle_deg: f32) {
        let angle = angle_deg.clamp(0.0, self.max_angle);
        self.angle = angle;
        let sys_hz = clk_sys_freq();
        let us_per_cycle = 1_000_000.0 / sys_hz as f32;

        let pulse_us = self.min_us as f32
            + (self.max_us as f32 - self.min_us as f32) * (angle / self.max_angle);
        let cycles = (pulse_us / us_per_cycle) as u32;

        self.pwm.set_level(cycles);
    }

    /// Write angle with smooth movement
    pub async fn write_smooth(&mut self, target_angle: f32) {
        let start = self.angle;
        let delta = target_angle - start;
        let steps = (delta.abs() / MAX_SPEED) as u32;

        if steps == 0 {
            self.write(target_angle);
            return;
        }

        for i in 0..=steps {
            let angle = start + (delta * i as f32 / steps as f32);
            self.write(angle);
            Timer::after_millis(1).await;
        }
    }

    pub fn to_min(&mut self) {
        self.write(0.0);
    }

    pub fn center(&mut self) {
        self.write(self.max_angle / 2.0);
    }

    pub fn to_max(&mut self) {
        self.write(self.max_angle);
    }

    pub fn get_angle(&self) -> f32 {
        self.angle
    }
}

/// Servo controller using hardware PWM (single channel)
pub struct ServoFromPWM<'d> {
    pwm: embassy_rp::pwm::PwmOutput<'d>,
    /// Pulse width for 0° (in microseconds)
    min_us: u64,
    /// Pulse width for max_angle (in microseconds)
    max_us: u64,
    /// Maximum angle the servo supports
    max_angle: f32,
    /// Current servo angle
    pub angle: f32,
    /// Ticks per microsecond (for calculations)
    ticks_per_us: f32,
}

impl<'d> ServoFromPWM<'d> {
    /// Create a new PWM servo controller
    pub fn new(
        pwm: embassy_rp::pwm::PwmOutput<'d>,
        min_us: u64,
        max_us: u64,
        max_angle: f32,
    ) -> Self {
        // With divider of 64: 125MHz / 64 = 1.953125 MHz
        // Each tick is ~0.512 µs
        let ticks_per_us = 1.953125;

        Self {
            pwm,
            min_us,
            max_us,
            max_angle,
            angle: 0.0,
            ticks_per_us,
        }
    }

    /// Most common convenience constructor - 180° servo with standard timings
    pub fn new_standard_180(pwm: embassy_rp::pwm::PwmOutput<'d>) -> Self {
        Self::new(pwm, 570, 2620, 180.0)
    }

    /// Write angle in degrees (0.0 ..= max_angle)
    pub fn write(&mut self, angle_deg: f32) {
        let angle = angle_deg.clamp(0.0, self.max_angle);
        self.angle = angle;

        // Calculate pulse width in microseconds
        let pulse_us = self.min_us as f32
            + (self.max_us as f32 - self.min_us as f32) * (angle / self.max_angle);

        // Convert to compare value
        let compare = (pulse_us * self.ticks_per_us) as u16;

        let _ = self.pwm.set_duty_cycle(compare);
    }

    /// Write angle with smooth movement
    pub async fn write_smooth(&mut self, target_angle: f32) {
        let start = self.angle;
        let delta = target_angle - start;
        let steps = (delta.abs() / MAX_SPEED) as u32;

        if steps == 0 {
            self.write(target_angle);
            return;
        }

        for i in 0..=steps {
            let angle = start + (delta * i as f32 / steps as f32);
            self.write(angle);
            Timer::after_millis(1).await;
        }
    }

    pub fn to_min(&mut self) {
        self.write(0.0);
    }

    pub fn center(&mut self) {
        self.write(self.max_angle / 2.0);
    }

    pub fn to_max(&mut self) {
        self.write(self.max_angle);
    }

    pub fn get_angle(&self) -> f32 {
        self.angle
    }
}

/// Macro to initialize PWM servo pairs
macro_rules! init_pwm_servo {
    ($slice:expr, $pin_a:expr, $pin_b:expr, $config:expr) => {{
        let pwm = Pwm::new_output_ab($slice, $pin_a, $pin_b, $config);
        let (a, b) = pwm.split();
        (
            ServoFromPWM::new_standard_180(a.expect("PWM SERVO ALREADY TAKEN")),
            ServoFromPWM::new_standard_180(b.expect("PWM SERVO ALREADY TAKEN")),
        )
    }};
}

/// Container for all 18 servos
pub struct AllServos<'d> {
    pub servos: [ServoType<'d>; 18],
}

impl<'d> AllServos<'d> {
    /// Initialize all 18 servos
    pub fn new(
        pio0: Peri<'d, embassy_rp::peripherals::PIO0>,

        pwm_slice0: Peri<'d, embassy_rp::peripherals::PWM_SLICE0>,
        pwm_slice1: Peri<'d, embassy_rp::peripherals::PWM_SLICE1>,
        pwm_slice2: Peri<'d, embassy_rp::peripherals::PWM_SLICE2>,
        pwm_slice3: Peri<'d, embassy_rp::peripherals::PWM_SLICE3>,
        pwm_slice4: Peri<'d, embassy_rp::peripherals::PWM_SLICE4>,
        pwm_slice5: Peri<'d, embassy_rp::peripherals::PWM_SLICE5>,
        pwm_slice6: Peri<'d, embassy_rp::peripherals::PWM_SLICE6>,
        pwm_slice7: Peri<'d, embassy_rp::peripherals::PWM_SLICE7>,

        pin_0: Peri<'d, embassy_rp::peripherals::PIN_0>,
        pin_1: Peri<'d, embassy_rp::peripherals::PIN_1>,
        pin_2: Peri<'d, embassy_rp::peripherals::PIN_2>,
        pin_3: Peri<'d, embassy_rp::peripherals::PIN_3>,
        pin_4: Peri<'d, embassy_rp::peripherals::PIN_4>,
        pin_5: Peri<'d, embassy_rp::peripherals::PIN_5>,
        pin_6: Peri<'d, embassy_rp::peripherals::PIN_6>,
        pin_7: Peri<'d, embassy_rp::peripherals::PIN_7>,
        pin_8: Peri<'d, embassy_rp::peripherals::PIN_8>,
        pin_9: Peri<'d, embassy_rp::peripherals::PIN_9>,
        pin_10: Peri<'d, embassy_rp::peripherals::PIN_10>,
        pin_11: Peri<'d, embassy_rp::peripherals::PIN_11>,
        pin_12: Peri<'d, embassy_rp::peripherals::PIN_12>,
        pin_13: Peri<'d, embassy_rp::peripherals::PIN_13>,
        pin_14: Peri<'d, embassy_rp::peripherals::PIN_14>,
        pin_15: Peri<'d, embassy_rp::peripherals::PIN_15>,
        pin_16: Peri<'d, embassy_rp::peripherals::PIN_16>,
        pin_17: Peri<'d, embassy_rp::peripherals::PIN_17>,
    ) -> Self {
        // Initialize PIO servos
        let Pio {
            common: mut common0,
            sm0,
            sm1,
            ..
        } = Pio::new(pio0, Irqs);

        let prg0 = PioPwmProgram::new(&mut common0);
        let pwm_pio1 = PioPwm::new(&mut common0, sm0, pin_16, &prg0);
        let pwm_pio2 = PioPwm::new(&mut common0, sm1, pin_17, &prg0);

        let mut pio1 = ServoFromPIO::new_standard_180(pwm_pio1);
        let mut pio2 = ServoFromPIO::new_standard_180(pwm_pio2);
        pio1.start();
        pio2.start();

        // Configure PWM for 50Hz servo signal
        let mut config = PwmConfig::default();
        config.divider = 64.into();
        config.top = 39062;

        // Initialize PWM servos using macro
        let (pwm0a, pwm0b) = init_pwm_servo!(pwm_slice0, pin_0, pin_1, config.clone());
        let (pwm1a, pwm1b) = init_pwm_servo!(pwm_slice1, pin_2, pin_3, config.clone());
        let (pwm2a, pwm2b) = init_pwm_servo!(pwm_slice2, pin_4, pin_5, config.clone());
        let (pwm3a, pwm3b) = init_pwm_servo!(pwm_slice3, pin_6, pin_7, config.clone());
        let (pwm4a, pwm4b) = init_pwm_servo!(pwm_slice4, pin_8, pin_9, config.clone());
        let (pwm5a, pwm5b) = init_pwm_servo!(pwm_slice5, pin_10, pin_11, config.clone());
        let (pwm6a, pwm6b) = init_pwm_servo!(pwm_slice6, pin_12, pin_13, config.clone());
        let (pwm7a, pwm7b) = init_pwm_servo!(pwm_slice7, pin_14, pin_15, config.clone());

        Self {
            servos: [
                ServoType::PWM(pwm0a),
                ServoType::PWM(pwm0b),
                ServoType::PWM(pwm1a),
                ServoType::PWM(pwm1b),
                ServoType::PWM(pwm2a),
                ServoType::PWM(pwm2b),
                ServoType::PWM(pwm3a),
                ServoType::PWM(pwm3b),
                ServoType::PWM(pwm4a),
                ServoType::PWM(pwm4b),
                ServoType::PWM(pwm5a),
                ServoType::PWM(pwm5b),
                ServoType::PWM(pwm6a),
                ServoType::PWM(pwm6b),
                ServoType::PWM(pwm7a),
                ServoType::PWM(pwm7b),
                ServoType::PIO(pio1),
                ServoType::PIO1(pio2),
            ],
        }
    }

    /// Move all servos to center position
    pub fn center_all(&mut self) {
        self.write_all(90.0);
    }

    pub fn zero_all(&mut self) {
        self.write_all(0.0);
    }

    pub fn max_all(&mut self) {
        self.write_all(180.0);
    }

    pub fn write_all(&mut self, angle: f32) {
        for s in &mut self.servos {
            s.write(angle);
        }
    }
}

pub enum ServoType<'d> {
    PIO(ServoFromPIO<'d, embassy_rp::peripherals::PIO0, 0>),
    PIO1(ServoFromPIO<'d, embassy_rp::peripherals::PIO0, 1>),
    PWM(ServoFromPWM<'d>),
}

impl<'d> ServoType<'d> {
    pub fn write(&mut self, angle: f32) {
        match self {
            ServoType::PIO(s) => s.write(angle),
            ServoType::PIO1(s) => s.write(angle),
            ServoType::PWM(s) => s.write(angle),
        }
    }

    pub async fn write_smooth(&mut self, angle: f32) {
        match self {
            ServoType::PIO(s) => s.write_smooth(angle).await,
            ServoType::PIO1(s) => s.write_smooth(angle).await,
            ServoType::PWM(s) => s.write_smooth(angle).await,
        }
    }

    pub fn center(&mut self) {
        match self {
            ServoType::PIO(s) => s.center(),
            ServoType::PIO1(s) => s.center(),
            ServoType::PWM(s) => s.center(),
        }
    }

    pub fn get_angle(&self) -> f32 {
        match self {
            ServoType::PIO(s) => s.get_angle(),
            ServoType::PIO1(s) => s.get_angle(),
            ServoType::PWM(s) => s.get_angle(),
        }
    }
}
