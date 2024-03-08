use rgb_int::Rgb24;

pub const FLOOR: Rgb24 = Rgb24::new(108, 0, 162);
pub const FLOOR_BLOODY: Rgb24 = Rgb24::new(255, 0, 0);
pub const VAPORWAVE_FOREGROUND: Rgb24 = Rgb24::new(255, 128, 255);
pub const VAPORWAVE_BACKGROUND: Rgb24 = Rgb24::new(68, 0, 102);
pub const CORRUPTION: Rgb24 = Rgb24::new(0, 255, 255);
pub const STAIRS: Rgb24 = Rgb24::new(0, 153, 230);
pub const ZOMBIE: Rgb24 = Rgb24::new(255, 51, 0);
pub const CLIMBER: Rgb24 = Rgb24::new(153, 255, 51);
pub const TRESPASSER: Rgb24 = Rgb24::new(0, 153, 255);
pub const BOOMER: Rgb24 = Rgb24::hex(0xcc7a00);
pub const SNATCHER: Rgb24 = Rgb24::hex(0xe6b800);
pub const POISONER: Rgb24 = Rgb24::hex(0x66ff33);
pub const DIVIDER: Rgb24 = Rgb24::hex(0x00cc99);
pub const CORRUPTOR: Rgb24 = Rgb24::hex(0x00ffff);
pub const SHOP_BG: Rgb24 = Rgb24::hex(0xcc00cc);
pub const SHOP_ITEM: Rgb24 = Rgb24::hex(0xffff00);
pub const SHOP_GUN: Rgb24 = Rgb24::hex(0x00ff00);
pub const SHOP_ORGAN_CLINIC: Rgb24 = Rgb24::hex(0x00ffff);
pub const NORMAL_MODE: Rgb24 = Rgb24::new(0, 255, 255);
pub const AIMING_MODE: Rgb24 = Rgb24::new(255, 0, 0);
pub const HEALTH: Rgb24 = Rgb24::hex(0x800000);
pub const OXYGEN: Rgb24 = Rgb24::hex(0x234790);
pub const FOOD: Rgb24 = Rgb24::hex(0x997300);
pub const POISON: Rgb24 = Rgb24::hex(0x336600);
pub const RADIATION: Rgb24 = Rgb24::hex(0x009973);
pub const GLOWER: Rgb24 = Rgb24::hex(0x009973);
pub const VENTER: Rgb24 = Rgb24::hex(0xcccccc);
pub const POWER: Rgb24 = Rgb24::hex(0xff00ff);
pub const SATIATION: Rgb24 = Rgb24::hex(0xff0000);
pub const MONEY: Rgb24 = Rgb24::hex(0xffff66);
pub const STIMPACK: Rgb24 = HEALTH.saturating_scalar_mul_div(2, 1);
pub const ANTIDOTE: Rgb24 = POISON;
pub const BLOOD_VIAL_EMPTY: Rgb24 = Rgb24::hex(0xadc2eb);
pub const BLOOD_VIAL_FULL: Rgb24 = OXYGEN;
pub const BATTERY: Rgb24 = POWER;
pub const ANTIRADS: Rgb24 = RADIATION;
pub const ORGAN_CONTAINER: Rgb24 = Rgb24::hex(0x00e6e6);
pub const PISTOL: Rgb24 = Rgb24::hex(0xbfbfbf);
pub const SHOTGUN: Rgb24 = Rgb24::hex(0xb35900);
pub const ROCKET_LAUNCHER: Rgb24 = Rgb24::hex(0xff471a);
