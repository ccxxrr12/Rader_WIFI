//! Edge Module Engine — 竞赛 WASM 模块原生集成
//!
//! 精简实现 10 个边缘模块的核心逻辑，在模拟/硬件模式下统一运行。
//! 量产部署时这些模块运行在 ESP32 的 WASM3 解释器中；
//! 竞赛演示期直接编译为原生 Rust 运行在 RZ/V2H 服务端。

use serde::{Deserialize, Serialize};

// ══════════════════════════════════════════════════════════════════════════════
// Shared Types
// ══════════════════════════════════════════════════════════════════════════════

/// A single alert produced by an edge module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAlert {
    pub module: String,
    pub event_type: i32,
    pub event_name: String,
    pub value: f32,
    pub severity: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// Module-specific state and logic
// ══════════════════════════════════════════════════════════════════════════════

// ── Ring Buffer ─────────────────────────────────────────────────────────────

struct RingBuf<const N: usize> {
    buf: [f32; N],
    idx: usize,
    len: usize,
}

impl<const N: usize> RingBuf<N> {
    fn new() -> Self { Self { buf: [0.0; N], idx: 0, len: 0 } }
    fn push(&mut self, v: f32) {
        self.buf[self.idx] = v;
        self.idx = (self.idx + 1) % N;
        if self.len < N { self.len += 1; }
    }
    fn mean_last(&self, n: usize) -> f32 {
        let c = n.min(self.len);
        if c == 0 { return 0.0; }
        let mut s = 0.0;
        for i in 0..c {
            let ri = (self.idx + N - c + i) % N;
            s += self.buf[ri];
        }
        s / c as f32
    }
    fn trend(&self, n: usize) -> f32 {
        let c = n.min(self.len);
        if c < 4 { return 0.0; }
        let q = c / 4;
        let (mut f, mut l) = (0.0f32, 0.0f32);
        for i in 0..q { f += self.buf[(self.idx + N - c + i) % N]; }
        for i in (c - q)..c { l += self.buf[(self.idx + N - c + i) % N]; }
        (l / q as f32 - f / q as f32) / c as f32
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Main Engine
// ══════════════════════════════════════════════════════════════════════════════

pub struct EdgeModuleEngine {
    // Module 1: vital_trend — 生命体征趋势
    vt_br: RingBuf<300>,      // breathing rate history (5 min @ 1Hz)
    vt_hr: RingBuf<300>,      // heart rate history
    vt_timer: u32,
    vt_apnea_ctr: u32,
    vt_brady_ctr: u8, vt_tachy_ctr: u8,
    vt_hr_brady_ctr: u8, vt_hr_tachy_ctr: u8,

    // Module 2: lrn_anomaly_attractor — 混沌吸引子
    att_center: [f32; 4], att_radius: f32,
    att_initialized: bool, att_frame: u32,
    att_cooldown: u16,
    att_lyap_sum: f64, att_lyap_cnt: u32,
    att_prev_state: [f32; 4], att_prev_delta: f32,

    // Module 3: coherence — CSI 相干性
    coh_prev_phases: [f32; 32],
    coh_smoothed: f32, coh_initialized: bool,
    coh_gate: u8, // 0=accept, 1=warn, 2=reject

    // Module 4: med_respiratory_distress — 呼吸窘迫
    rd_br_buf: RingBuf<60>,
    rd_var_buf: RingBuf<60>,
    rd_baseline_var: f32, rd_baseline_frames: u32,
    rd_tachy_ctr: u8, rd_cooldown: u16,
    rd_ac_buf: RingBuf<120>, rd_ac_frames: u32,

    // Module 5: ind_confined_space — 密闭空间监护
    cs_present: bool, cs_breathing_ok: bool,
    cs_no_br_ctr: u32, cs_no_motion_ctr: u32,
    cs_entry_logged: bool,

    // Module 6: sec_panic_motion — 恐慌动作检测
    pm_energy_buf: RingBuf<100>,
    pm_var_buf: RingBuf<100>,
    pm_cooldown: u16,

    // Module 7: med_sleep_apnea — 睡眠呼吸暂停
    sa_no_br_ctr: u32, sa_apnea_active: bool,
    sa_apnea_events: u32, sa_sleep_secs: u32,

    // Module 8: med_cardiac_arrhythmia — 心律失常
    ca_hr_buf: RingBuf<60>,
    ca_tachy_ctr: u8, ca_brady_ctr: u8,
    ca_missed_ctr: u8, ca_prev_hr: f32,

    // Module 9: med_seizure_detect — 癫痫检测
    sz_energy_buf: RingBuf<200>,
    sz_band_buf: RingBuf<200>,
    sz_seizing: bool, sz_seizure_ctr: u32,
    sz_postictal_ctr: u32,

    // Module 10: intrusion — 入侵检测
    intr_baseline: [f32; 32], intr_baseline_set: bool,
    intr_baseline_frames: u32, intr_armed: bool,
    intr_detect_ctr: u8, intr_cooldown: u16,
}

impl EdgeModuleEngine {
    pub fn new() -> Self {
        Self {
            vt_br: RingBuf::new(), vt_hr: RingBuf::new(), vt_timer: 0,
            vt_apnea_ctr: 0, vt_brady_ctr: 0, vt_tachy_ctr: 0,
            vt_hr_brady_ctr: 0, vt_hr_tachy_ctr: 0,
            att_center: [0.0; 4], att_radius: 0.0,
            att_initialized: false, att_frame: 0, att_cooldown: 0,
            att_lyap_sum: 0.0, att_lyap_cnt: 0,
            att_prev_state: [0.0; 4], att_prev_delta: 0.0,
            coh_prev_phases: [0.0; 32], coh_smoothed: 1.0,
            coh_initialized: false, coh_gate: 0,
            rd_br_buf: RingBuf::new(), rd_var_buf: RingBuf::new(),
            rd_baseline_var: 0.0, rd_baseline_frames: 0,
            rd_tachy_ctr: 0, rd_cooldown: 0,
            rd_ac_buf: RingBuf::new(), rd_ac_frames: 0,
            cs_present: false, cs_breathing_ok: false,
            cs_no_br_ctr: 0, cs_no_motion_ctr: 0, cs_entry_logged: false,
            pm_energy_buf: RingBuf::new(), pm_var_buf: RingBuf::new(),
            pm_cooldown: 0,
            sa_no_br_ctr: 0, sa_apnea_active: false,
            sa_apnea_events: 0, sa_sleep_secs: 0,
            ca_hr_buf: RingBuf::new(),
            ca_tachy_ctr: 0, ca_brady_ctr: 0, ca_missed_ctr: 0, ca_prev_hr: 0.0,
            sz_energy_buf: RingBuf::new(), sz_band_buf: RingBuf::new(),
            sz_seizing: false, sz_seizure_ctr: 0, sz_postictal_ctr: 0,
            intr_baseline: [0.0; 32], intr_baseline_set: false,
            intr_baseline_frames: 0, intr_armed: false,
            intr_detect_ctr: 0, intr_cooldown: 0,
        }
    }

    /// Process one CSI frame through all 10 modules.
    /// Returns aggregated alerts.
    #[allow(clippy::too_many_arguments)]
    pub fn process_frame(
        &mut self,
        phases: &[f32], amplitudes: &[f32],
        motion_energy: f32,
        breathing_bpm: Option<f64>, heart_rate_bpm: Option<f64>,
        presence: bool,
    ) -> Vec<EdgeAlert> {
        let mut alerts = Vec::new();
        let br = breathing_bpm.unwrap_or(0.0) as f32;
        let hr = heart_rate_bpm.unwrap_or(0.0) as f32;

        // Compute amplitude stats
        let n = amplitudes.len().min(32);
        let amp_mean = if n > 0 { amplitudes[..n].iter().sum::<f32>() / n as f32 } else { 0.0 };
        let amp_var = if n > 1 {
            amplitudes[..n].iter().map(|a| (a - amp_mean).powi(2)).sum::<f32>() / n as f32
        } else { 0.0 };

        // ── Module 1: vital_trend ──────────────────────────────────────
        self.vt_timer += 1;
        self.vt_br.push(br);
        self.vt_hr.push(hr);
        if self.vt_timer % 10 == 0 { // ~1 Hz
            // Apnea
            if br < 1.0 { self.vt_apnea_ctr += 1; }
            else { self.vt_apnea_ctr = 0; }
            if self.vt_apnea_ctr >= 20 {
                alerts.push(EdgeAlert {
                    module: "vital_trend".into(), event_type: 105,
                    event_name: "Apnea".into(), value: self.vt_apnea_ctr as f32,
                    severity: "critical".into(),
                });
            }
            // Bradypnea
            if br > 0.0 && br < 12.0 { self.vt_brady_ctr = self.vt_brady_ctr.saturating_add(1); }
            else { self.vt_brady_ctr = 0; }
            if self.vt_brady_ctr >= 5 {
                alerts.push(EdgeAlert { module: "vital_trend".into(), event_type: 101,
                    event_name: "Bradypnea".into(), value: br, severity: "warning".into() });
            }
            // Tachypnea
            if br > 25.0 { self.vt_tachy_ctr = self.vt_tachy_ctr.saturating_add(1); }
            else { self.vt_tachy_ctr = 0; }
            if self.vt_tachy_ctr >= 5 {
                alerts.push(EdgeAlert { module: "vital_trend".into(), event_type: 102,
                    event_name: "Tachypnea".into(), value: br, severity: "warning".into() });
            }
            // Bradycardia
            if hr > 0.0 && hr < 50.0 { self.vt_hr_brady_ctr = self.vt_hr_brady_ctr.saturating_add(1); }
            else { self.vt_hr_brady_ctr = 0; }
            if self.vt_hr_brady_ctr >= 5 {
                alerts.push(EdgeAlert { module: "vital_trend".into(), event_type: 103,
                    event_name: "Bradycardia".into(), value: hr, severity: "warning".into() });
            }
            // Tachycardia
            if hr > 120.0 { self.vt_hr_tachy_ctr = self.vt_hr_tachy_ctr.saturating_add(1); }
            else { self.vt_hr_tachy_ctr = 0; }
            if self.vt_hr_tachy_ctr >= 5 {
                alerts.push(EdgeAlert { module: "vital_trend".into(), event_type: 104,
                    event_name: "Tachycardia".into(), value: hr, severity: "critical".into() });
            }
        }

        // ── Module 2: lrn_anomaly_attractor ────────────────────────────
        if n > 0 {
            let state = [amplitudes[..n].iter().sum::<f32>() / n as f32,
                         br, amp_var.sqrt(), motion_energy];
            self.att_frame += 1;
            if self.att_cooldown > 0 { self.att_cooldown -= 1; }
            if !self.att_initialized {
                if self.att_frame == 1 { self.att_center = state; }
                else { for d in 0..4 { self.att_center[d] = 0.01 * state[d] + 0.99 * self.att_center[d]; } }
                let dist = euclid_4(&state, &self.att_center);
                if dist > self.att_radius { self.att_radius = dist; }
                // Compute Lyapunov contribution
                if self.att_frame > 1 {
                    let delta = euclid_4(&state, &self.att_prev_state);
                    if self.att_prev_delta > 1e-8 && delta > 1e-8 {
                        self.att_lyap_sum += (delta / self.att_prev_delta).ln() as f64;
                        self.att_lyap_cnt += 1;
                    }
                    self.att_prev_delta = delta;
                }
                self.att_prev_state = state;
                if self.att_frame >= 200 && self.att_lyap_cnt > 0 {
                    self.att_initialized = true;
                    if self.att_radius < 0.01 { self.att_radius = 0.01; }
                    alerts.push(EdgeAlert { module: "attractor".into(), event_type: 738,
                        event_name: "LearningComplete".into(), value: 1.0, severity: "info".into() });
                }
            } else {
                let dist = euclid_4(&state, &self.att_center);
                if dist > self.att_radius * 3.0 && self.att_cooldown == 0 {
                    self.att_cooldown = 100;
                    alerts.push(EdgeAlert { module: "attractor".into(), event_type: 737,
                        event_name: "BasinDeparture".into(), value: dist / self.att_radius,
                        severity: "critical".into() });
                }
            }
        }

        // ── Module 3: coherence ────────────────────────────────────────
        if n > 0 {
            if !self.coh_initialized {
                for i in 0..n.min(32) { self.coh_prev_phases[i] = phases[i]; }
                self.coh_initialized = true;
            } else {
                let (mut sum_re, mut sum_im) = (0.0f32, 0.0f32);
                for i in 0..n.min(32) {
                    let delta = phases[i] - self.coh_prev_phases[i];
                    sum_re += delta.cos(); sum_im += delta.sin();
                    self.coh_prev_phases[i] = phases[i];
                }
                let nf = n.min(32) as f32;
                let coh = ((sum_re/nf).powi(2) + (sum_im/nf).powi(2)).sqrt();
                self.coh_smoothed = 0.1 * coh + 0.9 * self.coh_smoothed;
                self.coh_gate = if self.coh_smoothed < 0.4 { 2 }
                               else if self.coh_smoothed < 0.7 { 1 }
                               else { 0 };
            }
        }

        // ── Module 4: med_respiratory_distress ─────────────────────────
        self.rd_br_buf.push(br);
        self.rd_var_buf.push(amp_var);
        if self.rd_cooldown > 0 { self.rd_cooldown -= 1; }
        // Baseline learning (60s)
        if self.rd_baseline_frames < 600 {
            self.rd_baseline_frames += 1;
            self.rd_baseline_var = (self.rd_baseline_var * (self.rd_baseline_frames - 1) as f32
                + amp_var) / self.rd_baseline_frames as f32;
        } else {
            // Tachypnea
            if br > 25.0 { self.rd_tachy_ctr = self.rd_tachy_ctr.saturating_add(1); }
            else { self.rd_tachy_ctr = 0; }
            if self.rd_tachy_ctr >= 8 && self.rd_cooldown == 0 {
                self.rd_cooldown = 400;
                alerts.push(EdgeAlert { module: "resp_distress".into(), event_type: 120,
                    event_name: "Tachypnea".into(), value: br, severity: "warning".into() });
            }
            // Labored breathing
            let recent_var = self.rd_var_buf.mean_last(30);
            if recent_var > self.rd_baseline_var * 3.0 && br > 0.0 {
                alerts.push(EdgeAlert { module: "resp_distress".into(), event_type: 121,
                    event_name: "LaboredBreathing".into(), value: recent_var / self.rd_baseline_var.max(0.001),
                    severity: "warning".into() });
            }
            // Cheyne-Stokes (autocorrelation-based, every 30s)
            self.rd_ac_buf.push(amp_var);
            self.rd_ac_frames += 1;
            if self.rd_ac_frames % 300 == 0 && self.rd_ac_frames >= 900 {
                let ac = autocorr_max(&self.rd_ac_buf, 30, 90);
                if ac > 0.35 {
                    alerts.push(EdgeAlert { module: "resp_distress".into(), event_type: 122,
                        event_name: "CheyneStokes".into(), value: ac, severity: "critical".into() });
                }
            }
        }

        // ── Module 5: ind_confined_space ───────────────────────────────
        let was_present = self.cs_present;
        self.cs_present = presence;
        self.cs_breathing_ok = br > 0.0;
        // Entry/exit
        if self.cs_present && !was_present && self.cs_entry_logged {
            alerts.push(EdgeAlert { module: "confined_space".into(), event_type: 510,
                event_name: "WorkerEntry".into(), value: 1.0, severity: "info".into() });
        }
        if !self.cs_present && was_present {
            alerts.push(EdgeAlert { module: "confined_space".into(), event_type: 511,
                event_name: "WorkerExit".into(), value: 1.0, severity: "info".into() });
            self.cs_no_br_ctr = 0; self.cs_no_motion_ctr = 0;
        }
        if self.cs_present { self.cs_entry_logged = true; }
        // No breathing → extraction alert
        if self.cs_present && !self.cs_breathing_ok {
            self.cs_no_br_ctr += 1;
            if self.cs_no_br_ctr > 300 { // 15s @ 20Hz
                alerts.push(EdgeAlert { module: "confined_space".into(), event_type: 513,
                    event_name: "ExtractionAlert".into(), value: self.cs_no_br_ctr as f32 / 20.0,
                    severity: "critical".into() });
            }
        } else { self.cs_no_br_ctr = 0; }
        // Immobile → immobile alert
        if self.cs_present && motion_energy < 0.1 {
            self.cs_no_motion_ctr += 1;
            if self.cs_no_motion_ctr > 1200 { // 60s @ 20Hz
                alerts.push(EdgeAlert { module: "confined_space".into(), event_type: 514,
                    event_name: "ImmobileAlert".into(), value: self.cs_no_motion_ctr as f32 / 20.0,
                    severity: "critical".into() });
            }
        } else { self.cs_no_motion_ctr = 0; }

        // ── Module 6: sec_panic_motion ─────────────────────────────────
        if self.pm_cooldown > 0 { self.pm_cooldown -= 1; }
        self.pm_energy_buf.push(motion_energy);
        self.pm_var_buf.push(amp_var);
        if self.pm_cooldown == 0 && self.pm_energy_buf.len >= 100 {
            let jerk = jerk_estimate(&self.pm_energy_buf);
            let entropy = entropy_estimate(&self.pm_var_buf);
            let mean_energy = self.pm_energy_buf.mean_last(100);
            if jerk > 2.0 && entropy > 0.35 && mean_energy > 1.0 && presence {
                self.pm_cooldown = 100;
                alerts.push(EdgeAlert { module: "panic_motion".into(), event_type: 250,
                    event_name: "PanicDetected".into(), value: jerk, severity: "critical".into() });
            } else if jerk > 1.5 && entropy > 0.25 && mean_energy < 5.0 && presence {
                alerts.push(EdgeAlert { module: "panic_motion".into(), event_type: 251,
                    event_name: "StrugglePattern".into(), value: entropy, severity: "warning".into() });
            } else if mean_energy > 5.0 && jerk > 0.05 && entropy < 0.25 {
                alerts.push(EdgeAlert { module: "panic_motion".into(), event_type: 252,
                    event_name: "FleeingDetected".into(), value: mean_energy, severity: "warning".into() });
            }
        }

        // ── Module 7: med_sleep_apnea ──────────────────────────────────
        self.sa_sleep_secs += 1;
        if br < 4.0 { self.sa_no_br_ctr += 1; }
        else {
            if self.sa_apnea_active {
                alerts.push(EdgeAlert { module: "sleep_apnea".into(), event_type: 101,
                    event_name: "ApneaEnd".into(), value: self.sa_no_br_ctr as f32 / 20.0,
                    severity: "info".into() });
            }
            self.sa_no_br_ctr = 0;
            self.sa_apnea_active = false;
        }
        if self.sa_no_br_ctr > 200 && !self.sa_apnea_active { // 10s @ 20Hz
            self.sa_apnea_active = true;
            self.sa_apnea_events += 1;
            alerts.push(EdgeAlert { module: "sleep_apnea".into(), event_type: 100,
                event_name: "ApneaStart".into(), value: self.sa_no_br_ctr as f32 / 20.0,
                severity: "critical".into() });
        }
        // AHI report every hour
        if self.sa_sleep_secs % 72000 == 0 { // 3600s * 20Hz
            let ahi = self.sa_apnea_events as f32 / (self.sa_sleep_secs as f32 / 72000.0);
            alerts.push(EdgeAlert { module: "sleep_apnea".into(), event_type: 102,
                event_name: "AHIUpdate".into(), value: ahi, severity: "info".into() });
        }

        // ── Module 8: med_cardiac_arrhythmia ───────────────────────────
        self.ca_hr_buf.push(hr);
        let avg_hr = self.ca_hr_buf.mean_last(30);
        // Tachycardia
        if hr > 100.0 { self.ca_tachy_ctr = self.ca_tachy_ctr.saturating_add(1); }
        else { self.ca_tachy_ctr = 0; }
        if self.ca_tachy_ctr >= 40 { // 2s sustained
            alerts.push(EdgeAlert { module: "cardiac".into(), event_type: 110,
                event_name: "Tachycardia".into(), value: hr, severity: "warning".into() });
        }
        // Bradycardia
        if hr > 0.0 && hr < 50.0 { self.ca_brady_ctr = self.ca_brady_ctr.saturating_add(1); }
        else { self.ca_brady_ctr = 0; }
        if self.ca_brady_ctr >= 40 {
            alerts.push(EdgeAlert { module: "cardiac".into(), event_type: 111,
                event_name: "Bradycardia".into(), value: hr, severity: "warning".into() });
        }
        // Missed beat
        if self.ca_prev_hr > 10.0 && hr < self.ca_prev_hr * 0.7 {
            self.ca_missed_ctr = self.ca_missed_ctr.saturating_add(1);
            if self.ca_missed_ctr >= 3 {
                alerts.push(EdgeAlert { module: "cardiac".into(), event_type: 112,
                    event_name: "MissedBeat".into(), value: hr, severity: "critical".into() });
                self.ca_missed_ctr = 0;
            }
        } else { self.ca_missed_ctr = 0; }
        self.ca_prev_hr = hr;
        // HRV anomaly (simple RMSSD)
        if self.ca_hr_buf.len >= 30 {
            let rmssd = rmssd(&self.ca_hr_buf, 30);
            if rmssd > 100.0 || (rmssd < 10.0 && avg_hr > 0.0) {
                alerts.push(EdgeAlert { module: "cardiac".into(), event_type: 113,
                    event_name: "HRVAnomaly".into(), value: rmssd, severity: "warning".into() });
            }
        }

        // ── Module 9: med_seizure_detect ───────────────────────────────
        self.sz_energy_buf.push(motion_energy);
        // Estimate 3-8 Hz band energy via amplitude variance oscillation
        let band_energy = amp_var; // simplified proxy
        self.sz_band_buf.push(band_energy);
        if !self.sz_seizing {
            let mean_e = self.sz_energy_buf.mean_last(60);
            let mean_b = self.sz_band_buf.mean_last(60);
            if mean_e > 3.0 && mean_b > 2.0 && presence {
                self.sz_seizing = true;
                self.sz_seizure_ctr = 0;
                self.sz_postictal_ctr = 0;
                alerts.push(EdgeAlert { module: "seizure".into(), event_type: 140,
                    event_name: "SeizureOnset".into(), value: mean_e, severity: "critical".into() });
            }
        } else {
            self.sz_seizure_ctr += 1;
            let mean_e = self.sz_energy_buf.mean_last(20);
            // Tonic phase (high energy, low variance)
            if mean_e > 5.0 && self.sz_band_buf.mean_last(20) < 1.0 {
                alerts.push(EdgeAlert { module: "seizure".into(), event_type: 141,
                    event_name: "SeizureTonic".into(), value: mean_e, severity: "critical".into() });
            }
            // Clonic phase (rhythmic 3-8Hz)
            if self.sz_band_buf.mean_last(20) > 2.0 {
                alerts.push(EdgeAlert { module: "seizure".into(), event_type: 142,
                    event_name: "SeizureClonic".into(), value: mean_e, severity: "critical".into() });
            }
            // Recovery
            if motion_energy < 1.0 && self.sz_seizure_ctr > 100 {
                self.sz_postictal_ctr += 1;
                if self.sz_postictal_ctr > 40 {
                    self.sz_seizing = false;
                    alerts.push(EdgeAlert { module: "seizure".into(), event_type: 143,
                        event_name: "PostIctal".into(), value: 1.0, severity: "warning".into() });
                }
            } else { self.sz_postictal_ctr = 0; }
        }

        // ── Module 10: intrusion ───────────────────────────────────────
        if self.intr_cooldown > 0 { self.intr_cooldown -= 1; }
        if !self.intr_baseline_set {
            if n > 0 {
                for i in 0..n.min(32) { self.intr_baseline[i] += amplitudes[i]; }
                self.intr_baseline_frames += 1;
            }
            if self.intr_baseline_frames >= 200 {
                for b in self.intr_baseline.iter_mut() { *b /= self.intr_baseline_frames as f32; }
                self.intr_baseline_set = true;
            }
        } else {
            // Arm when quiet for 5s
            if motion_energy < 0.5 && !presence {
                if !self.intr_armed {
                    self.intr_armed = true;
                    alerts.push(EdgeAlert { module: "intrusion".into(), event_type: 202,
                        event_name: "IntrusionArmed".into(), value: 1.0, severity: "info".into() });
                }
            }
            // Detect intrusion
            if self.intr_armed && presence && self.intr_cooldown == 0 {
                if n > 0 {
                    let mut change = 0.0f32;
                    for i in 0..n.min(32) {
                        if self.intr_baseline[i] > 0.1 {
                            change += (amplitudes[i] / self.intr_baseline[i]).abs();
                        }
                    }
                    change /= n as f32;
                    if change > 3.0 {
                        self.intr_detect_ctr = self.intr_detect_ctr.saturating_add(1);
                        if self.intr_detect_ctr >= 3 {
                            self.intr_cooldown = 100;
                            self.intr_detect_ctr = 0;
                            alerts.push(EdgeAlert { module: "intrusion".into(), event_type: 200,
                                event_name: "IntrusionAlert".into(), value: change,
                                severity: "critical".into() });
                        }
                    } else { self.intr_detect_ctr = 0; }
                }
            }
        }

        alerts
    }

    /// Get coherence status.
    pub fn coherence_status(&self) -> &'static str {
        match self.coh_gate { 0 => "accept", 1 => "warn", _ => "reject" }
    }
    pub fn coherence_score(&self) -> f32 { self.coh_smoothed }
}

// ══════════════════════════════════════════════════════════════════════════════
// Math helpers
// ══════════════════════════════════════════════════════════════════════════════

fn euclid_4(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    ((a[0]-b[0]).powi(2) + (a[1]-b[1]).powi(2) + (a[2]-b[2]).powi(2) + (a[3]-b[3]).powi(2)).sqrt()
}

fn jerk_estimate(buf: &RingBuf<100>) -> f32 {
    if buf.len < 3 { return 0.0; }
    let mut max_jerk = 0.0f32;
    let idx = buf.idx;
    for i in 2..buf.len {
        let v0 = buf.buf[(idx + 100 - i + 0) % 100];
        let v1 = buf.buf[(idx + 100 - i + 1) % 100];
        let v2 = buf.buf[(idx + 100 - i + 2) % 100];
        let j = (v2 - 2.0 * v1 + v0).abs(); // second derivative
        if j > max_jerk { max_jerk = j; }
    }
    max_jerk * 20.0 // scale to Hz
}

fn entropy_estimate(buf: &RingBuf<100>) -> f32 {
    if buf.len < 2 { return 0.0; }
    let mut reversals = 0u32;
    let idx = buf.idx;
    for i in 1..buf.len {
        let prev = buf.buf[(idx + 100 - i - 1) % 100];
        let curr = buf.buf[(idx + 100 - i + 0) % 100];
        let next = if i + 1 < buf.len { buf.buf[(idx + 100 - i + 1) % 100] } else { curr };
        if (curr - prev) * (next - curr) < 0.0 { reversals += 1; }
    }
    reversals as f32 / buf.len as f32
}

fn autocorr_max(buf: &RingBuf<120>, lag_min: usize, lag_max: usize) -> f32 {
    if buf.len < lag_max { return 0.0; }
    let n = buf.len;
    let mean = buf.mean_last(n);
    let mut variance = 0.0f32;
    let idx = buf.idx;
    for i in 0..n {
        let v = buf.buf[(idx + 120 - n + i) % 120] - mean;
        variance += v * v;
    }
    if variance < 1e-8 { return 0.0; }
    let mut max_ac = 0.0f32;
    for lag in lag_min..=lag_max.min(n - 1) {
        let mut cov = 0.0f32;
        for i in 0..(n - lag) {
            let a = buf.buf[(idx + 120 - n + i) % 120] - mean;
            let b = buf.buf[(idx + 120 - n + i + lag) % 120] - mean;
            cov += a * b;
        }
        let ac = cov / variance;
        if ac > max_ac { max_ac = ac; }
    }
    max_ac
}

fn rmssd(buf: &RingBuf<60>, n: usize) -> f32 {
    let c = n.min(buf.len);
    if c < 2 { return 0.0; }
    let idx = buf.idx;
    let mut ss = 0.0f64;
    let mut cnt = 0u32;
    for i in 1..c {
        let a = buf.buf[(idx + 60 - c + i - 1) % 60] as f64;
        let b = buf.buf[(idx + 60 - c + i) % 60] as f64;
        ss += (b - a).powi(2);
        cnt += 1;
    }
    if cnt == 0 { 0.0 } else { (ss / cnt as f64).sqrt() as f32 }
}
