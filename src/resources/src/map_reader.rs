use crate::map_structs::*;
use crate::reader::BinaryDataReader;
use std::io;
use std::path::Path;

// this logic is heavily based on the VCMI implementation
//
pub fn load_h3m(filename: &Path) -> io::Result<Map> {
    if !filename.exists() {
        panic!("load_h3m: file {filename:?} does not exist");
    }

    if !filename.is_file() {
        panic!("load_h3m: {filename:?} is not a file");
    }
    let mut reader = BinaryDataReader::new_possibly_gzip(std::fs::read(&filename)?)?;
    Ok(parse_map(&mut reader)?)
}

fn parse_map(reader: &mut BinaryDataReader) -> io::Result<Map> {
    let map_format = reader.read_u32_le()?;
    let format = match Format::from(map_format) {
        Some(f) => f,
        None => return Err(gen_error("Unknown map format")),
    };

    let mut ctx = ParsingContext::from(format, 0);

    if format == Format::HOTA {
        // TODO: HOTA format not supported yet
        let hota_version = reader.read_u32_le()?;
        ctx = ParsingContext::from(format, hota_version as usize);
        if hota_version > 0 {
            let _is_mirror_map = reader.read_bool()?;
            let _is_arena_map = reader.read_bool()?;
        }
        if hota_version > 1 {
            let _ = reader.read_u32_le(); // always equal to 12?
        }
    }
    let any_players = reader.read_bool()?;
    let map_dimension = reader.read_i32_le()?;
    let two_levels = reader.read_bool()?;
    let name = reader.read_string_le()?;
    let description = reader.read_string_le()?;
    let difficulty = match Difficulty::from(reader.read_u8()?) {
        Some(d) => d,
        None => return Err(gen_error("Unknown difficulty level")),
    };
    let hero_level_limit = if ctx.level_AB {
        Some(reader.read_u8()? as usize)
    } else {
        None
    };
    let info = Info {
        format,
        any_players,
        map_dimension,
        two_levels,
        name,
        description,
        difficulty,
        hero_level_limit,
    };
    let players = parse_player_info(reader, &ctx)?;
    // read victory loss conditions
    // read team info
    // read all allowed heroes
    // read disposed heroes
    // read map options
    // read allowed artifacts
    // read allowed spell abilities
    // read rumors
    // read predefined heroes
    // read terrain
    // read object templates
    // read objects
    // read events
    Ok(Map { info, players })
}

fn parse_player_info(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Vec<PlayerInfo>> {
    use Player::*;
    let mut players = Vec::new();
    for player in [Red, Blue, Tan, Green, Orange, Purple, Teal, Pink] {
        let player = player;
        let can_be_human = reader.read_bool()?;
        let can_be_computer = reader.read_bool()?;
        if !(can_be_human || can_be_computer) {
            // inactive player
            if ctx.level_ROE {
                reader.skip_n(6);
            }
            if ctx.level_AB {
                reader.skip_n(6)
            }
            if ctx.level_SOD {
                reader.skip_n(1);
            }
            players.push(PlayerInfo {
                player,
                can_be_human,
                can_be_computer,
                ..Default::default()
            });
            continue;
        }
        let behaviour = match PlayerBehaviour::from(reader.read_i8()?) {
            Some(b) => b,
            None => return Err(gen_error("Unknown player behaviour")),
        };
        if ctx.level_SOD {
            reader.skip_n(1);
        }
        // read factions
        use Town::*;
        // this is true only for ROE
        let mut towns = vec![
            Castle, Rampart, Tower, Inferno, Necropolis, Dungeon, Stronghold, Fortress,
        ];
        if ctx.level_AB || ctx.level_SOD || ctx.level_WOG || ctx.level == Format::HOTA {
            towns.push(Conflux);
        }
        if ctx.level == Format::HOTA {
            towns.push(Cove);
        }
        // TODO: add support for factory (HOTA4?)
        let faction_towns = map_bits_to_objects(reader, &towns)?;
        let is_faction_random = reader.read_bool()?;
        let all_allowed = is_faction_random && faction_towns.len() == towns.len();
        let faction;
        if all_allowed {
            faction = Faction::RandomAll;
        } else if is_faction_random && faction_towns.len() > 0 {
            faction = Faction::RandomSome(faction_towns);
        } else if faction_towns.len() == 1 {
            faction = Faction::Some(faction_towns[0]);
        } else {
            faction = Faction::None;
        }
        // main town
        let has_main_town = reader.read_bool()?;
        let main_town_position;
        let mut generate_hero_at_main_town = true;
        if has_main_town {
            if ctx.level_AB {
                generate_hero_at_main_town = reader.read_bool()?;
                // Type of town: FF - Random town, others correspond to 0 - Castle etc.
                reader.skip_n(1);
            }
            main_town_position = Some((
                reader.read_u8()? as usize,
                reader.read_u8()? as usize,
                reader.read_u8()? as usize,
            ));
        } else {
            main_town_position = None;
        }
        // player's heroes
        // lead hero
        let has_random_hero = reader.read_bool()?;
        let hero_type_id = reader.read_u8()?;
        let lead_hero = if hero_type_id != 0xff {
            let portrait_id = Some(reader.read_u8()?);
            let name = reader.read_string_le()?;
            Some(Hero {
                id: hero_type_id,
                portrait_id,
                name,
            })
        } else {
            None
        };
        // other heroes
        let mut other_heroes = Vec::new();
        if ctx.level_AB {
            reader.skip_n(1);
            let hero_count = reader.read_u32_le()?;
            for _ in 0..hero_count {
                let id = reader.read_u8()?;
                let name = reader.read_string_le()?;
                other_heroes.push(Hero {
                    id,
                    portrait_id: None,
                    name,
                })
            }
        }
        players.push(PlayerInfo {
            player,
            can_be_human,
            can_be_computer,
            behaviour,
            faction,
            generate_hero_at_main_town,
            main_town_position,
            has_random_hero,
            lead_hero,
            other_heroes,
        });
    }
    Ok(players)
}

/// Read a byte and collect an item from `object`
/// if corresponding bit is set
fn map_bits_to_objects<T: Copy + Clone>(
    reader: &mut BinaryDataReader,
    objects: &[T],
) -> io::Result<Vec<T>> {
    let mut ret = Vec::new();
    let mut mask = 0u8;
    for (i, o) in objects.iter().enumerate() {
        let bit_no = i % 8;
        if bit_no == 0 {
            mask = reader.read_u8()?;
        }

        if mask & (1 << bit_no) != 0 {
            ret.push(*o);
        }
    }
    Ok(ret)
}

fn gen_error(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, msg)
}

#[allow(non_snake_case)]
#[derive(Default)]
struct ParsingContext {
    // factions_bytes: usize,
    heroes_bytes: usize,
    artifacts_bytes: usize,
    resources_bytes: usize,
    skills_bytes: usize,
    spells_bytes: usize,
    buildings_bytes: usize,

    // total number of elements of appropriate type
    // factions_count: usize,
    heroes_count: usize,
    heroes_portraits_count: usize,
    artifacts_count: usize,
    resources_count: usize,
    creatures_count: usize,
    spells_count: usize,
    skills_count: usize,
    terrains_count: usize,
    roads_count: usize,
    rivers_count: usize,
    artifact_slots_count: usize,
    buildings_count: usize,

    // identifier that should be treated as "invalid", usually - '-1'
    hero_identifier_invalid: usize,
    artifact_identifier_invalid: usize,
    creature_identifier_invalid: usize,
    spell_identifier_invalid: usize,

    // features from which map format are available
    level: Format,
    level_ROE: bool,
    level_AB: bool,
    level_SOD: bool,
    level_WOG: bool,
    level_HOTA0: bool,
    level_HOTA1: bool,
    level_HOTA3: bool,
}

impl ParsingContext {
    fn from(map_format: Format, hota_version: usize) -> Self {
        let mut ctx = Self::default();
        ctx.level = map_format;
        // Format::ROE
        ctx.level_ROE = true;

        // ctx.factions_bytes = 1;
        ctx.heroes_bytes = 16;
        ctx.artifacts_bytes = 16;
        ctx.skills_bytes = 4;
        ctx.resources_bytes = 4;
        ctx.spells_bytes = 9;
        ctx.buildings_bytes = 6;

        // ctx.factions_count = 8;
        ctx.heroes_count = 128;
        ctx.heroes_portraits_count = 130; // +General Kendal, +Catherine (portrait-only in RoE)
        ctx.artifacts_count = 127;
        ctx.resources_count = 7;
        ctx.creatures_count = 118;
        ctx.spells_count = 70;
        ctx.skills_count = 28;
        ctx.terrains_count = 10;
        ctx.artifact_slots_count = 18;
        ctx.buildings_count = 41;
        ctx.roads_count = 3;
        ctx.rivers_count = 4;

        ctx.hero_identifier_invalid = 0xff;
        ctx.artifact_identifier_invalid = 0xff;
        ctx.creature_identifier_invalid = 0xff;
        ctx.spell_identifier_invalid = 0xff;
        // AB
        if [
            Format::AB,
            Format::SOD,
            Format::WOG,
            Format::HOTA,
            Format::VCMI,
        ]
        .contains(&map_format)
        {
            ctx.level_AB = true;

            // ctx.factions_bytes = 2; // + Conflux
            // ctx.factions_count = 9;

            ctx.creatures_count = 145; // + Conflux and new neutrals

            ctx.heroes_count = 156; // + Conflux and campaign heroes
            ctx.heroes_portraits_count = 159; // +_kendal, +young Cristian, +Ordwald
            ctx.heroes_bytes = 20;

            ctx.artifacts_count = 129; // + Armaggedon Blade and Vial of Dragon Blood
            ctx.artifacts_bytes = 17;

            ctx.artifact_identifier_invalid = 0xffff; // Now uses 2 bytes / object
            ctx.creature_identifier_invalid = 0xffff; // Now uses 2 bytes / object
        }
        // SOD
        if [Format::SOD, Format::WOG, Format::HOTA, Format::VCMI].contains(&map_format) {
            ctx.level_SOD = true;

            ctx.artifacts_count = 144; // + _combined artifacts + 3 unfinished artifacts (required for some maps)
            ctx.artifacts_bytes = 18;

            ctx.heroes_portraits_count = 163; // +Finneas +young Gem +young Sandro +young Yog

            ctx.artifact_slots_count = 19; // + MISC_5 slot
        }

        // HOTA
        if [Format::HOTA].contains(&map_format) {
            ctx.level_HOTA0 = true;
            ctx.level_HOTA1 = hota_version > 0;
            //ctxresult.levelHOTA2 = hotaVersion > 1; // HOTA2 seems to be identical to HOTA1 so far
            ctx.level_HOTA3 = hota_version > 2;

            ctx.artifacts_bytes = 21;
            ctx.heroes_bytes = 23;

            ctx.terrains_count = 12; // +Highlands +Wasteland
            ctx.skills_count = 29; // + Interference
                                   // ctx.factions_count = 10; // + Cove
            ctx.creatures_count = 171; // + Cove + neutrals

            if hota_version < 3 {
                ctx.artifacts_count = 163; // + HotA artifacts
                ctx.heroes_count = 178; // + Cove
                ctx.heroes_portraits_count = 186; // + Cove
            }
            if hota_version == 3 {
                ctx.artifacts_count = 165; // + HotA artifacts
                ctx.heroes_count = 179; // + Cove + Giselle
                ctx.heroes_portraits_count = 188; // + Cove + Giselle
            }
        }
        // WOG
        if [Format::WOG].contains(&map_format) {
            ctx.level_WOG = true;
        }
        if [Format::VCMI].contains(&map_format) {
            unimplemented!();
        }

        ctx
    }
}
