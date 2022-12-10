use binrw::{binrw, NullString};

use crate::{jgr::SLXI, Gamma};

#[binrw]
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

#[binrw]
#[brw(big)]
#[br(import { slxi: &SLXI })]
#[derive(Debug)]
pub struct City {
    xy: u32,
    townnamegrfid: u32,
    townnametype: u16,
    townnameparts: u32,
    #[br(temp)]
    #[bw(calc = Gamma { value: (name.len()).try_into().unwrap() })]
    name_size: Gamma,
    #[br(count = name_size.value, map = |x: Vec<u8>| String::from_utf8_lossy(&x).to_string())]
    #[bw(map = |x: &String| x.as_bytes())]
    name: String,
    flags: u8,
    #[br(if(slxi.has_feature("town_multi_building")))]
    church_count: Option<u16>,
    #[br(if(slxi.has_feature("town_multi_building")))]
    stadium_count: Option<u16>,
    statues: u16,
    have_ratings: u16,
    #[br(temp)]
    #[bw(calc = Gamma { value: (ratings.len()).try_into().unwrap() })]
    ratings_size: Gamma,
    #[br(count = ratings_size.value)]
    ratings: Vec<u16>,
    #[br(temp)]
    #[bw(calc = Gamma { value: (unwanted.len()).try_into().unwrap() })]
    unwanted_size: Gamma,
    #[br(count = unwanted_size.value)]
    unwanted: Vec<u8>,
    #[br(temp)]
    #[bw(calc = Gamma { value: (goal.len()).try_into().unwrap() })]
    goal_size: Gamma,
    #[br(count = goal_size.value)]
    goal: Vec<u32>,
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
    // #[br(temp)]
    // #[bw(calc = Gamma { value: (psa_list.len()).try_into().unwrap() })]
    // psa_list_size: Gamma,
    // #[br(count = psa_list_size.value)]
    // psa_list: Vec<u32>,
    // #[br(temp)]
    // #[bw(calc = Gamma { value: (supplied.len()).try_into().unwrap() })]
    // supplied_size: Gamma,
    // #[br(count = supplied_size.value)]
    // supplied: Vec<Supplied>,
    // #[br(temp)]
    // #[bw(calc = Gamma { value: (received.len()).try_into().unwrap() })]
    // received_size: Gamma,
    // #[br(count = received_size.value)]
    // received: Vec<Received>,
}
