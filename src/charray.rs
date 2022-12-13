use binrw::binrw;

use crate::{gamma::Gamma, jgr::SLXI};

#[binrw]
#[brw(big)]
#[derive(Debug)]
pub struct Supplied {
    old_max: u32,
    new_max: u32,
    old_act: u32,
    new_act: u32,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
pub struct Received {
    old_max: u16,
    new_max: u16,
    old_act: u16,
    new_act: u16,
}

const NUM_TE: usize = 6;
const MAX_COMPANIES: usize = 0x0F;
const MAX_CARGO: usize = 64;

#[binrw]
#[brw(big)]
#[brw(import { slxi: &SLXI })]
#[derive(Debug)]
pub struct City {
    pub xy: u32,
    townnamegrfid: u32,
    townnametype: u16,
    townnameparts: u32,
    #[br(temp)]
    #[bw(calc = Gamma { value: (name.len()).try_into().unwrap() })]
    name_size: Gamma,
    #[br(count = name_size.value, map = |x: Vec<u8>| String::from_utf8_lossy(&x).to_string())]
    #[bw(map = |x: &String| x.as_bytes())]
    pub name: String,
    flags: u8,
    #[brw(if(slxi.has_feature("town_multi_building")))]
    church_count: Option<u16>,
    #[brw(if(slxi.has_feature("town_multi_building")))]
    stadium_count: Option<u16>,
    statues: u16,
    have_ratings: u16,
    ratings: [u16; MAX_COMPANIES],
    unwanted: [u8; MAX_COMPANIES],
    goal: [u32; NUM_TE],
    #[br(temp)]
    #[bw(calc = Gamma { value: (text.len()).try_into().unwrap() })]
    text_size: Gamma,
    #[br(count = text_size.value, map = |x: Vec<u8>| String::from_utf8_lossy(&x).to_string())]
    #[bw(map = |x: &String| x.as_bytes())]
    text: String,
    time_until_rebuild: u16,
    grow_counter: u16,
    growth_rate: u16,
    fund_buildings_months: u8,
    road_build_months: u8,
    exclusivity: u8,
    exlusive_counter: u8,
    larger_town: i8,
    layout: u8,
    #[br(temp)]
    #[bw(calc = psa_list.len().try_into().unwrap())]
    psa_list_size: u32,
    #[br(count = psa_list_size)]
    psa_list: Vec<u32>,
    #[brw(if(slxi.has_feature("town_setting_override")))]
    override_flags: u8,
    #[brw(if(slxi.has_feature("town_setting_override")))]
    override_values: u8,
    #[brw(if(slxi.has_feature("town_setting_override")))]
    build_tunnels: u8,
    #[brw(if(slxi.has_feature("town_setting_override")))]
    max_road_slope: u8,
    supplied: [Supplied; MAX_CARGO],
    received: [Received; NUM_TE],
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
pub struct Maps {
    pub dim_x: u32,
    pub dim_y: u32,
}
