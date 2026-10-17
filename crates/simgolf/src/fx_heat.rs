//! Enhanced graphics' heat waves (docs/ENHANCED.md, "Heat waves"; the port's own): on the desert's hot summer days a warm,
//! bleached light through the midday hours, a warm haze over the distance, and on the hottest days the grass parched
//! towards straw, green again once the heat breaks. Which days are hot is sg_core::weather's hash of the course and the
//! day, the days the Weather that matters extra plays. Drawing only and cheap: it changes the light's colours, the haze
//! and the terrain shader's golden grade (the autumn's, fx_glsl.rs `seasonal`), with no pass of its own, so there is
//! nothing for the frame-rate guard to step down.
use crate::app::App;
use crate::fx_light::{grey, lerp3, mul3, Atmos};
use sg_core::weather::smoothstep;

/// The bleached, warm sky of a hot day.
const HOT_SKY: [f32; 3] = [0.96, 0.91, 0.78];
/// The heat haze's colour.
const HOT_HAZE: [f32; 3] = [1.0, 0.93, 0.80];

impl App {
    /// A heat wave of strength `heat` (0..1) into this moment's atmosphere; `phase` the time of day's (None without it:
    /// a steady midday-ish look).
    pub(crate) fn fx_heat(&self, a: &mut Atmos, heat: f32, phase: Option<f32>) {
        // strongest through the midday hours, a little of it morning and evening
        let mid = phase.map(|p| smoothstep(0.22, 0.38, p) * (1.0 - smoothstep(0.62, 0.8, p))).unwrap_or(0.75);
        let k = heat.clamp(0.0, 1.0) * (0.35 + 0.65 * mid);
        // a warmer, brighter sun; the colours a little washed out
        a.sun = mul3(a.sun, [1.0 + 0.10 * k, 1.0 + 0.04 * k, 1.0 - 0.10 * k]);
        a.amb = grey(a.amb, 0.15 * k).map(|v| v * (1.0 + 0.06 * k));
        a.sprite = mul3(grey(a.sprite, 0.12 * k), [1.0 + 0.06 * k, 1.0 + 0.03 * k, 1.0 - 0.04 * k]);
        a.sky_top = lerp3(a.sky_top, HOT_SKY, 0.35 * k);
        a.sky_low = lerp3(a.sky_low, HOT_SKY, 0.45 * k);
        // the haze over the distance, warm (fx_light.rs `fx_overlay` draws it)
        a.haze = a.haze.max(0.25 + 0.3 * k);
        a.haze_col = lerp3(a.sky_low, HOT_HAZE, 0.5);
        a.tinted = true;
        a.heat = heat;
        // the grass parches on the hottest days only (it follows the day's heat, so it greens again after)
        a.parched = 0.9 * smoothstep(0.7, 0.95, heat);
    }
}
