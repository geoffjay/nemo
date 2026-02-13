/// Standard PID controller with anti-windup clamping.
pub struct PidController {
    /// Proportional gain.
    pub kp: f64,
    /// Integral gain.
    pub ki: f64,
    /// Derivative gain.
    pub kd: f64,
    /// Accumulated integral term.
    integral: f64,
    /// Previous error (for derivative).
    prev_error: f64,
    /// Minimum output clamp.
    output_min: f64,
    /// Maximum output clamp.
    output_max: f64,
}

#[allow(dead_code)]
impl PidController {
    /// Creates a new PID controller with the given gains.
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
            output_min: f64::NEG_INFINITY,
            output_max: f64::INFINITY,
        }
    }

    /// Sets output clamping limits.
    pub fn with_output_limits(mut self, min: f64, max: f64) -> Self {
        self.output_min = min;
        self.output_max = max;
        self
    }

    /// Updates gains for live tuning.
    pub fn set_gains(&mut self, kp: f64, ki: f64, kd: f64) {
        self.kp = kp;
        self.ki = ki;
        self.kd = kd;
    }

    /// Computes the PID output for the given setpoint and process variable.
    ///
    /// `dt` is the time step in seconds. Returns the clamped control output.
    pub fn compute(&mut self, setpoint: f64, process_variable: f64, dt: f64) -> f64 {
        let error = setpoint - process_variable;

        // Proportional term
        let p = self.kp * error;

        // Integral term with anti-windup: only accumulate if output is not saturated
        self.integral += error * dt;
        let i = self.ki * self.integral;

        // Derivative term (on error)
        let d = if dt > 0.0 {
            self.kd * (error - self.prev_error) / dt
        } else {
            0.0
        };
        self.prev_error = error;

        let output = p + i + d;

        // Clamp and apply anti-windup
        let clamped = output.clamp(self.output_min, self.output_max);

        // Anti-windup: if output is saturated, back-calculate the integral
        if (clamped - output).abs() > f64::EPSILON {
            self.integral -= error * dt;
        }

        clamped
    }

    /// Resets the controller state (integral and previous error).
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p_only() {
        let mut pid = PidController::new(2.0, 0.0, 0.0);
        let output = pid.compute(10.0, 5.0, 0.1);
        // P = 2.0 * (10 - 5) = 10.0
        assert!((output - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_i_accumulation() {
        let mut pid = PidController::new(0.0, 1.0, 0.0);
        // First step: error=5, integral=5*0.1=0.5
        let out1 = pid.compute(10.0, 5.0, 0.1);
        assert!((out1 - 0.5).abs() < 1e-9);

        // Second step: error=5, integral=0.5+0.5=1.0
        let out2 = pid.compute(10.0, 5.0, 0.1);
        assert!((out2 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_d_response() {
        let mut pid = PidController::new(0.0, 0.0, 1.0);
        // First step: error=5, prev_error=0, D = 1.0*(5-0)/0.1 = 50
        let out1 = pid.compute(10.0, 5.0, 0.1);
        assert!((out1 - 50.0).abs() < 1e-9);

        // Second step: error=3, prev_error=5, D = 1.0*(3-5)/0.1 = -20
        let out2 = pid.compute(10.0, 7.0, 0.1);
        assert!((out2 - -20.0).abs() < 1e-9);
    }

    #[test]
    fn test_anti_windup() {
        let mut pid = PidController::new(0.0, 10.0, 0.0).with_output_limits(-5.0, 5.0);

        // Large error should clamp to 5.0 and not accumulate integral
        let out1 = pid.compute(100.0, 0.0, 1.0);
        assert!((out1 - 5.0).abs() < 1e-9);

        // After clamping, integral should not have wound up
        let out2 = pid.compute(100.0, 0.0, 1.0);
        assert!((out2 - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_reset() {
        let mut pid = PidController::new(1.0, 1.0, 1.0);
        pid.compute(10.0, 5.0, 0.1);

        pid.reset();
        // After reset, integral and prev_error should be zero
        // So output = P + I + D = 1*5 + 1*(5*0.1) + 1*(5-0)/0.1
        // = 5 + 0.5 + 50 = 55.5
        let out = pid.compute(10.0, 5.0, 0.1);
        assert!((out - 55.5).abs() < 1e-9);
    }

    #[test]
    fn test_set_gains() {
        let mut pid = PidController::new(1.0, 0.0, 0.0);
        let out1 = pid.compute(10.0, 5.0, 0.1);
        assert!((out1 - 5.0).abs() < 1e-9);

        pid.reset();
        pid.set_gains(3.0, 0.0, 0.0);
        let out2 = pid.compute(10.0, 5.0, 0.1);
        assert!((out2 - 15.0).abs() < 1e-9);
    }

    #[test]
    fn test_zero_dt() {
        let mut pid = PidController::new(1.0, 1.0, 1.0);
        // dt=0 should not cause division by zero; D term should be 0
        let out = pid.compute(10.0, 5.0, 0.0);
        // P=5, I=0 (5*0=0), D=0
        assert!((out - 5.0).abs() < 1e-9);
    }
}
