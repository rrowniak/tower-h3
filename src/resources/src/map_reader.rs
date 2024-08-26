use crate::map_buildings::*;
use crate::map_obj_type::*;
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
    let win_loss_cond = parse_win_loss_cond(reader, &ctx)?;
    // println!("{win_loss_cond:?}");
    // read team info
    let team_info = parse_team_info(reader, &ctx)?;
    // println!("{team_info:?}");
    // read all allowed heroes & read disposed heroes
    let heroes_def = parse_heroes_def(reader, &ctx)?;
    // println!("{heroes_def:?}");
    // read map options
    let map_options = parse_map_options(reader, &ctx)?;
    // println!("{map_options:?}");
    // read allowed artifacts
    let allowed_artifacts = parse_allowed_artifacts(reader, &ctx)?;
    // read allowed spell abilities
    let allowed_spells = parse_allowed_spells(reader, &ctx)?;
    // read rumors
    let rumors = parse_rumors(reader, &ctx)?;
    // read predefined heroes
    let predefined_heroes = parse_predefined_heroes(reader, &ctx)?;
    // println!("{predefined_heroes:?}");
    // read terrain
    let levels_no = if info.two_levels { 2 } else { 1 };
    let map_size = info.map_dimension as usize;
    let terrains = parse_terrain(reader, &ctx, levels_no, map_size)?;
    // read object templates
    let object_templates = parse_object_templates(reader, &ctx)?;
    // println!("{object_templates:?}");
    // read objects
    let objects = parse_objects(reader, &ctx, &object_templates)?;
    // read events
    let events = parse_events(reader, &ctx)?;
    Ok(Map {
        info,
        players,
        win_loss_cond,
        team_info,
        heroes_def,
        map_options,
        allowed_artifacts,
        allowed_spells,
        rumors,
        predefined_heroes,
        terrains,
        object_templates,
        objects,
        events,
    })
}

fn parse_player_info(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Vec<PlayerInfo>> {
    let mut players = Vec::new();
    for player in ALL_PLAYERS {
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
        // TODO: add support for factory (HOTA4?)
        let faction_towns = read_bitmask_factions(reader, ctx)?;
        let is_faction_random = reader.read_bool()?;
        let all_allowed = is_faction_random && faction_towns.len() == ctx.factions.len();
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
            main_town_position = Some(read_coord(reader)?);
        } else {
            main_town_position = None;
        }
        // player's heroes
        // lead hero
        let has_random_hero = reader.read_bool()?;
        let hero_type_id = reader.read_u8()?;
        let lead_hero = if hero_type_id != ctx.hero_identifier_invalid {
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

fn parse_win_loss_cond(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<WinLossCond> {
    use SpecialVictoryCondition::*;
    let vict_code = reader.read_u8()?;
    let (allow_normal_victory, victory_cond_applies_to_comp) = if vict_code != 0xff {
        (reader.read_bool()?, reader.read_bool()?)
    } else {
        (false, false)
    };

    let special_victory_cond = match vict_code {
        0 => {
            // artifact
            let artifact_code = match read_artifact_id(reader, ctx)? {
                Some(id) => id,
                None => {
                    return Err(gen_error(
                        "Parsing artifact id for special victory condition failed",
                    ))
                }
            };
            Some(AcquireArtifact { artifact_code })
        }
        1 => {
            // gather creatures
            let unit_code = if ctx.level_AB {
                reader.read_u16_le()?
            } else {
                reader.read_u8()? as u16
            };
            let amount = reader.read_u32_le()?;
            Some(AccumulateCreatures { unit_code, amount })
        }
        2 => {
            // gather resources
            let resource = match Resource::from(reader.read_u8()?) {
                Some(r) => r,
                None => return Err(gen_error("Unknown resource code id")),
            };
            let amount = reader.read_u32_le()?;
            Some(AccumulateResources { resource, amount })
        }
        3 => {
            // build city
            let town_coord = read_coord(reader)?;
            let hall_level = match reader.read_u8()? {
                0 => Building::Town,
                1 => Building::City,
                2 => Building::Capitol,
                _ => return Err(gen_error("can't decode hall level")),
            };
            let castle_level = match reader.read_u8()? {
                0 => Building::Fort,
                1 => Building::Citadel,
                2 => Building::Castle,
                _ => return Err(gen_error("can't decode hall level")),
            };
            Some(UpgradeTown {
                town_coord,
                hall_level,
                castle_level,
            })
        }
        4 => {
            // build grail
            let town_coord = read_coord(reader)?;
            Some(BuildGrail { town_coord })
        }
        5 => {
            // defeat hero
            let hero_coord = read_coord(reader)?;
            Some(DefeatHero { hero_coord })
        }
        6 => {
            // capture town
            let town_coord = read_coord(reader)?;
            Some(CaptureTown { town_coord })
        }
        7 => {
            // defeat monsters
            let monster_coord = read_coord(reader)?;
            Some(DefeatMonster { monster_coord })
        }
        8 => {
            // flag all dwellings
            Some(FlagAllCreatureDwellings)
        }
        9 => {
            // flag all mines
            Some(FlagAllMines)
        }
        10 => {
            // transport artifact
            let artifact_code = match read_artifact_id(reader, ctx)? {
                Some(id) => id,
                None => {
                    return Err(gen_error(
                        "parsing artifact id for 'transport artifact' failed",
                    ))
                }
            };
            let artifact_coord = read_coord(reader)?;
            Some(TransportArtifact {
                artifact_code,
                artifact_coord,
            })
        }
        11 => {
            // hota eliminate all monsters
            Some(EliminateAllMonsters)
        }
        12 => {
            // hota survive for N days
            let limit_days = reader.read_u32_le()?;
            Some(SurviveNDays { limit_days })
        }
        0xff => None,
        _ => None,
    };

    let loss_cond = reader.read_u8()?;
    use SpecialLossCondition::*;
    let special_loss_cond = match loss_cond {
        0 => {
            // loss town
            let town_coord = read_coord(reader)?;
            Some(LossTown { town_coord })
        }
        1 => {
            // loss hero
            let hero_coord = read_coord(reader)?;
            Some(LossHero { hero_coord })
        }
        2 => {
            // time expires
            let limit_days = reader.read_u16_le()? as u32;
            Some(TimeExpires { limit_days })
        }
        0xff => None,
        _ => None,
    };

    Ok(WinLossCond {
        allow_normal_victory,
        victory_cond_applies_to_comp,
        special_victory_cond,
        special_loss_cond,
    })
}

fn parse_team_info(reader: &mut BinaryDataReader, _ctx: &ParsingContext) -> io::Result<TeamInfo> {
    let team_num = reader.read_u8()?;
    let mut team_info = TeamInfo::new();
    if team_num > 0 {
        for player in ALL_PLAYERS {
            let team_num = reader.read_u8()?;
            team_info.add(team_num, player);
        }
    }
    Ok(team_info)
}

fn parse_heroes_def(reader: &mut BinaryDataReader, ctx: &ParsingContext) -> io::Result<HeroesDef> {
    // read allowed heroes
    let allowed_heroes = if ctx.level_HOTA0 {
        let heroes_count = reader.read_u32_le()?;
        assert!(heroes_count < 256);
        map_bits_to_numbers(reader, heroes_count as u8)?
    } else {
        map_bits_to_numbers(reader, ctx.heroes_count as u8)?
    };
    let mut reserved_for_campaign = Vec::new();
    if ctx.level_AB {
        let heroes_no = reader.read_u32_le()?;
        for _ in 0..heroes_no {
            let hero_id = reader.read_u8()?;
            reserved_for_campaign.push(hero_id);
        }
    }
    // read disposed heroes
    let mut disposed_heroes = Vec::new();
    if ctx.level_SOD {
        let disp_no = reader.read_u8()?;
        for _ in 0..disp_no {
            let id = reader.read_u8()?;
            let portrait_id = Some(reader.read_u8()?);
            let name = reader.read_string_le()?;
            let hero = Hero {
                id,
                portrait_id,
                name,
            };
            let players = map_bits_to_objects(reader, &ALL_PLAYERS, 1)?;
            disposed_heroes.push((hero, players));
        }
    }
    Ok(HeroesDef {
        allowed_heroes,
        reserved_for_campaign,
        disposed_heroes,
    })
}

fn parse_map_options(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<MapOptions> {
    reader.skip_n(31);
    let allow_special_months = if ctx.level_HOTA0 {
        let ret = reader.read_bool()?;
        reader.skip_n(3);
        ret
    } else {
        false
    };
    if ctx.level_HOTA1 {
        // unknown part
        let _ = reader.read_u8()?;
        reader.skip_n(5);
    }
    let round_limit = if ctx.level_HOTA3 {
        Some(reader.read_u32_le()?)
    } else {
        None
    };
    Ok(MapOptions {
        allow_special_months,
        round_limit,
    })
}

fn parse_allowed_artifacts(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<AllowedArtifacts> {
    let artifacts = if ctx.level_AB {
        if ctx.level_HOTA0 {
            let cnt = reader.read_u32_le()? as u8;
            map_bits_to_numbers(reader, cnt)?
                .iter()
                .map(|a| ArtifactId(*a as u32))
                .collect::<Vec<_>>()
        } else {
            map_bits_to_numbers(reader, ctx.artifacts_count as u8)?
                .iter()
                .map(|a| ArtifactId(*a as u32))
                .collect::<Vec<_>>()
        }
    } else {
        Vec::new()
    };
    Ok(AllowedArtifacts { artifacts })
}

fn parse_allowed_spells(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<AllowedSpells> {
    let (spells, skills) = if ctx.level_SOD {
        (
            map_bits_to_numbers(reader, ctx.spells_count as u8)?,
            map_bits_to_numbers(reader, ctx.skills_count as u8)?,
        )
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(AllowedSpells { spells, skills })
}

fn parse_rumors(reader: &mut BinaryDataReader, _ctx: &ParsingContext) -> io::Result<Rumors> {
    let cnt = reader.read_u32_le()?;
    let mut rumors = Vec::new();
    for _ in 0..cnt {
        let name = reader.read_string_le()?;
        let rumor = reader.read_string_le()?;
        rumors.push(Rumor { name, rumor });
    }
    Ok(Rumors { rumors })
}

fn parse_predefined_heroes(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Vec<PredefinedHero>> {
    let mut predefined_heroes = Vec::new();
    if !ctx.level_SOD {
        return Ok(predefined_heroes);
    }
    let heroes_cnt = if ctx.level_HOTA0 {
        reader.read_u32_le()? as usize
    } else {
        ctx.heroes_count
    };
    for id in 0..heroes_cnt {
        let custom = reader.read_bool()?;
        if !custom {
            continue;
        }
        let experience = if reader.read_bool()? {
            reader.read_u32_le()?
        } else {
            0
        };
        let secondary_skills = if reader.read_bool()? {
            let amount = reader.read_u32_le()?;
            let mut ret = Vec::new();
            for _ in 0..amount {
                ret.push(read_secondary_skill(reader, ctx)?);
            }
            ret
        } else {
            Vec::new()
        };
        let (artifacts, artifacts_in_bag) = read_heroes_artifacts(reader, ctx)?;
        let custom_bio = if reader.read_bool()? {
            Some(reader.read_string_le()?)
        } else {
            None
        };
        let gender = match reader.read_u8()? {
            0 => Some(Gender::Male),
            1 => Some(Gender::Female),
            _ => None,
        };
        let custom_spells = if reader.read_bool()? {
            // read custom spells
            map_bits_to_numbers(reader, ctx.spells_count as u8)?
        } else {
            Vec::new()
        };
        let custom_primary_skills = if reader.read_bool()? {
            Some(PrimarySkills {
                attack: reader.read_u8()? as u32,
                defence: reader.read_u8()? as u32,
                spell_power: reader.read_u8()? as u32,
                knowledge: reader.read_u8()? as u32,
            })
        } else {
            None
        };
        predefined_heroes.push(PredefinedHero {
            id: id as u32,
            experience,
            secondary_skills,
            artifacts,
            artifacts_in_bag,
            custom_bio,
            gender,
            custom_spells,
            custom_primary_skills,
        })
    }
    Ok(predefined_heroes)
}

fn read_coord(reader: &mut BinaryDataReader) -> io::Result<MapCoord> {
    Ok(MapCoord::from(
        reader.read_u8()?,
        reader.read_u8()?,
        reader.read_u8()?,
    ))
}

fn parse_terrain(
    reader: &mut BinaryDataReader,
    _ctx: &ParsingContext,
    levels_no: usize,
    map_size: usize,
) -> io::Result<Vec<Terrain>> {
    let mut terrains = Vec::new();
    for _ in 0..levels_no {
        let mut tiles = vec![vec![TerrainTile::default(); map_size]; map_size];
        for x in 0..map_size {
            for y in 0..map_size {
                let code = reader.read_u8()?;
                tiles[x][y].surface_type = match Surface::from(code) {
                    Some(t) => t,
                    None => return Err(gen_error(&format!("parse_terrain: cannot convert surface type - got {code} which is incorrect"))),
                };
                tiles[x][y].surface_picture = reader.read_u8()?;
                tiles[x][y].river_type = RiverType::from(reader.read_u8()?);
                tiles[x][y].river_direction = reader.read_u8()?;
                tiles[x][y].road_type = RoadType::from(reader.read_u8()?);
                tiles[x][y].road_direction = reader.read_u8()?;
                tiles[x][y].mirroring_flags = reader.read_u8()?;
            }
        }
        terrains.push(tiles);
    }
    Ok(terrains)
}

fn parse_object_templates(
    reader: &mut BinaryDataReader,
    _ctx: &ParsingContext,
) -> io::Result<Vec<ObjectTemplate>> {
    let mut ret = Vec::new();
    let amount = reader.read_u32_le()?;
    for _ in 0..amount {
        let animation_file = reader.read_string_le()?;
        // transit properties
        let mut block_bits = [0u8; 6];
        let mut visit_bits = [0u8; 6];
        for b in block_bits.iter_mut() {
            *b = reader.read_u8()?;
        }
        for b in visit_bits.iter_mut() {
            *b = reader.read_u8()?;
        }
        let mut transit_matrix = [[TileTransitProperty::default(); 8]; 6];
        for (i, row) in transit_matrix.iter_mut().enumerate() {
            for (j, trans) in row.iter_mut().enumerate() {
                // in h3m counted from the bottom right corner
                // remap to be counted from the top left corner
                let i = 5 - i;
                let j = 7 - j;
                if (block_bits[i] >> j) & 0x01 == 0 {
                    *trans = TileTransitProperty::TransitBlocked;
                }
                if (visit_bits[i] >> j) & 0x01 == 1 {
                    *trans = TileTransitProperty::Visitable;
                }
            }
        }
        // what kinds of landscape it can be put on - skip that
        let _ = reader.read_u16_le()?;
        // terrain mask
        let terrain_mask = reader.read_u16_le()?;
        let mut allowed_terrains = Vec::new();
        for bit_no in 0..16 {
            if terrain_mask & (1 << bit_no) != 0 {
                if let Some(s) = Surface::from(bit_no) {
                    allowed_terrains.push(s);
                }
            }
        }
        let id = reader.read_u32_le()?;
        let subid = reader.read_u32_le()?;
        let obj_kind = ObjectKind::from(reader.read_u8()?);
        let render_priority = reader.read_u8()?;
        reader.skip_n(16);
        ret.push(ObjectTemplate {
            animation_file,
            transit_matrix,
            allowed_terrains,
            id,
            subid,
            obj_kind,
            render_priority,
        });
    }
    Ok(ret)
}

fn parse_objects(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
    templates: &[ObjectTemplate],
) -> io::Result<Vec<Object>> {
    let mut ret = Vec::new();
    let amount = reader.read_u32_le()?;
    for _ in 0..amount {
        // reader.dump_hex(0, 16 * 4)?;
        let position = read_coord(reader)?;
        let obj_templ_id = reader.read_u32_le()?;
        reader.skip_n(5);
        let template = &templates[obj_templ_id as usize];
        let obj_id = template.id;
        println!("parsing {obj_id} at {position:?}...");
        let mut obj_type = match ObjectType::from(obj_id) {
            Some(o) => o,
            None => {
                return Err(gen_error(&format!(
                    "parse_objects: parsing object type failed, got {obj_templ_id}"
                )))
            }
        };
        use crate::map_obj_type::ObjectType::*;
        match obj_type {
            Monster(ref mut m)
            | RandomMonster(ref mut m)
            | RandomMonsterL1(ref mut m)
            | RandomMonsterL2(ref mut m)
            | RandomMonsterL3(ref mut m)
            | RandomMonsterL4(ref mut m)
            | RandomMonsterL5(ref mut m)
            | RandomMonsterL6(ref mut m)
            | RandomMonsterL7(ref mut m) => {
                // read moster data
                if ctx.level_AB {
                    m.id = reader.read_u32_le()?;
                }
                m.amount = reader.read_u16_le()? as u32;
                // println!("amount = {}", m.amount);
                m.character = reader.read_u8()?;
                if reader.read_bool()? {
                    m.message = Some(reader.read_string_le()?);
                    m.resources = Some(read_resource_pack(reader, ctx)?);
                    m.artifact = read_artifact_id(reader, ctx)?;
                } else {
                    m.message = None;
                }
                m.never_flees = reader.read_bool()?;
                m.growing_team = !reader.read_bool()?;
                reader.skip_n(2);
                if ctx.level_HOTA3 {
                    m.aggression_factor = match reader.read_u32_le()? {
                        0xffffffff => None,
                        v => Some(v),
                    };
                    m.join_only_for_money = Some(reader.read_bool()?);
                    m.join_percentage = Some(reader.read_u32_le()?);
                    m.upgraded_creatures = match reader.read_u32_le()? {
                        0xffffffff => None,
                        v => Some(v),
                    };
                    m.creatures_on_battle = match reader.read_u32_le()? {
                        0xffffffff => None,
                        v => Some(v),
                    };
                }
                // println!("{m:?}");
            }
            Event(ref mut ev) => {
                ev.box_content = Some(read_box_content(reader, ctx)?);
                ev.available_for = map_bits_to_objects(reader, &ALL_PLAYERS, 1)?;
                ev.computer_can_activate = reader.read_bool()?;
                ev.remove_after_visit = reader.read_bool()?;
                reader.skip_n(4);
                ev.human_can_activate = if ctx.level_HOTA3 {
                    reader.read_bool()?
                } else {
                    false
                };
            }
            Shipyard { ref mut owner }
            | Lighthouse { ref mut owner }
            | CreatureGenerator1 { ref mut owner }
            | CreatureGenerator2 { ref mut owner }
            | CreatureGenerator3 { ref mut owner }
            | CreatureGenerator4 { ref mut owner } => {
                *owner = Ownership::from(reader.read_u32_le()?);
            }
            Mine(ref mut m) | AbandonedMine(ref mut m) => {
                if template.subid < 7 {
                    m.owner = Ownership::from(reader.read_u32_le()?);
                } else {
                    m.abandoned_resources =
                        map_bits_to_objects(reader, &ALL_RESOURCES, ctx.resources_bytes)?;
                }
            }
            Hero(ref mut h) | RandomHero(ref mut h) | Prison(ref mut h) => {
                if ctx.level_AB {
                    h.quest_id = reader.read_u32_le()?;
                }
                h.owner = Ownership::from(reader.read_u8()? as u32);
                h.hero_id = reader.read_u8()? as u32;
                if reader.read_bool()? {
                    h.name = Some(reader.read_string_le()?);
                }
                if ctx.level_SOD {
                    if reader.read_bool()? {
                        h.experience = Some(reader.read_u32_le()?);
                    }
                } else {
                    let exp = reader.read_u32_le()?;
                    if exp > 0 {
                        h.experience = Some(exp);
                    }
                }
                if reader.read_bool()? {
                    h.portrait_id = Some(reader.read_u8()?);
                }
                if reader.read_bool()? {
                    let amount = reader.read_u32_le()?;
                    for _ in 0..amount {
                        h.secondary_skills.push(read_secondary_skill(reader, ctx)?);
                    }
                }
                if reader.read_bool()? {
                    h.garison = read_creature_set(reader, ctx)?;
                }
                h.army_formation = ArmyFormation::from(reader.read_u8()?);
                (h.artifacts, h.artifacts_in_bag) = read_heroes_artifacts(reader, ctx)?;
                h.patrol_radius = reader.read_u8()?;
                if ctx.level_AB {
                    if reader.read_bool()? {
                        h.custom_biography = Some(reader.read_string_le()?);
                    }
                    h.gender = match reader.read_u8()? {
                        0 => Some(Gender::Male),
                        1 => Some(Gender::Female),
                        _ => None,
                    };
                }
                if ctx.level_SOD {
                    if reader.read_bool()? {
                        h.custom_spells = map_bits_to_numbers(reader, ctx.spells_count as u8)?;
                    }
                } else if ctx.level_AB {
                    let spell_id = reader.read_u8()?;
                    if spell_id != ctx.spell_identifier_invalid {
                        h.custom_spells.push(spell_id);
                    }
                }
                if ctx.level_SOD {
                    if reader.read_bool()? {
                        h.custom_primary_skills = Some(PrimarySkills {
                            attack: reader.read_u8()? as u32,
                            defence: reader.read_u8()? as u32,
                            spell_power: reader.read_u8()? as u32,
                            knowledge: reader.read_u8()? as u32,
                        });
                    }
                }
                reader.skip_n(16);
            }
            Artifact(ref mut guards)
            | RandomArt(ref mut guards)
            | RandomTreasureArt(ref mut guards)
            | RandomMinorArt(ref mut guards)
            | RandomMajorArt(ref mut guards)
            | RandomRelicArt(ref mut guards) => {
                *guards = read_message_and_guards(reader, ctx)?;
            }
            SpellScroll(ref mut s) => {
                s.guards = read_message_and_guards(reader, ctx)?;
                s.spell_scroll_id = reader.read_u32_le()?;
            }
            Resource(ref mut r) | RandomResource(ref mut r) => {
                r.guards = read_message_and_guards(reader, ctx)?;
                r.amount = reader.read_u32_le()?;
                reader.skip_n(4);
            }
            Sign(ref mut msg) | OceanBottle(ref mut msg) => {
                *msg = reader.read_string_le()?;
                reader.skip_n(4);
            }
            SeerHut(ref mut vec) => {
                let mut quest_cnt = 1;
                if ctx.level_HOTA3 {
                    quest_cnt = reader.read_u32_le()?;
                }
                for _ in 0..quest_cnt {
                    vec.push(read_seer_hut_quest(reader, ctx)?);
                }
                if ctx.level_HOTA3 {
                    let repeateable_quests = reader.read_u32_le()?;
                    for _ in 0..repeateable_quests {
                        let mut q = read_seer_hut_quest(reader, ctx)?;
                        q.repeateable = true;
                        vec.push(q);
                    }
                }
                reader.skip_n(2);
            }
            WitchHut {
                ref mut secondary_skills,
            } => {
                if ctx.level_AB {
                    *secondary_skills = map_bits_to_numbers(reader, ctx.skills_count as u8)?;
                }
            }
            Scholar(ref mut s) => {
                let bonus_type = reader.read_u8()?;
                let bonus_id = reader.read_u8()?;
                *s = ScholarBonus::from(bonus_type, bonus_id);
                reader.skip_n(6);
            }
            Garrison(ref mut d) | Garrison2(ref mut d) => {
                d.owner = Ownership::from(reader.read_u32_le()?);
                d.guards = read_creature_set(reader, ctx)?;
                if ctx.level_AB {
                    d.removable_units = reader.read_bool()?;
                } else {
                    d.removable_units = true;
                }
                reader.skip_n(8);
            }
            Town(ref mut d) | RandomTown(ref mut d) => {
                if ctx.level_AB {
                    d.id = reader.read_u32_le()?;
                }
                d.owner = Ownership::from(reader.read_u8()? as u32);
                if reader.read_bool()? {
                    d.name = Some(reader.read_string_le()?);
                }
                if reader.read_bool()? {
                    d.guards = read_creature_set(reader, ctx)?;
                }
                d.army_formation = ArmyFormation::from(reader.read_u8()?);
                if reader.read_bool()? {
                    // custom buildings
                    d.built_buildings = read_bitmask_buildings(reader, ctx)?;
                    d.forbidden_buildings = read_bitmask_buildings(reader, ctx)?;
                } else {
                    // standard buildings
                    if reader.read_bool()? {
                        d.built_buildings.push(Buildings::Fort);
                    }
                    d.built_buildings.push(Buildings::Default);
                }
                if ctx.level_AB {
                    d.obligatory_spells = map_bits_to_numbers(reader, ctx.spells_count as u8)?;
                }
                d.possible_spells = map_bits_to_numbers(reader, ctx.spells_count as u8)?;
                if ctx.level_HOTA1 {
                    let _spells_research_available = reader.read_bool()?;
                }
                let events_cnt = reader.read_u32_le()?;
                for _ in 0..events_cnt {
                    let name = reader.read_string_le()?;
                    let message = reader.read_string_le()?;
                    let resources = read_resource_pack(reader, ctx)?;
                    let players = map_bits_to_objects(reader, &ALL_PLAYERS, 1)?;
                    let human_affected = if ctx.level_SOD {
                        reader.read_bool()?
                    } else {
                        true
                    };
                    let computer_affected = reader.read_bool()?;
                    let first_occurrence_at = reader.read_u16_le()?;
                    let next_occurrence = reader.read_u8()?;
                    reader.skip_n(17);
                    let new_buildings = read_bitmask_buildings(reader, ctx)?;
                    let mut new_creatures_at = Vec::with_capacity(7);
                    for i in 0..7 {
                        new_creatures_at.push((i as u8, reader.read_u16_le()?));
                    }
                    reader.skip_n(4);
                    d.events.push(TownEvent {
                        name,
                        message,
                        resources,
                        players,
                        human_affected,
                        computer_affected,
                        first_occurrence_at,
                        next_occurrence,
                        new_buildings,
                        new_creatures_at,
                    })
                }
                if ctx.level_SOD {
                    let alignment = reader.read_u8()?;
                    if alignment != 255 {
                        if (alignment as usize) < ALL_PLAYERS.len() {
                            d.alignment_to_player = Some(ALL_PLAYERS[alignment as usize]);
                        }
                    }
                }
                reader.skip_n(3);
            }
            ShrineOfMagicIncantation { ref mut spell_id }
            | ShrineOfMagicGesture { ref mut spell_id }
            | ShrineOfMagicThought { ref mut spell_id } => *spell_id = reader.read_u32_le()?,
            PandorasBox(ref mut pand_box) => *pand_box = read_box_content(reader, ctx)?,
            Grail { ref mut radius } => {
                if template.subid < 1000 {
                    *radius = reader.read_i32_le()?;
                }
            }
            RandomDwelling(ref mut d) => {
                d.owner = Ownership::from(reader.read_u32_le()?);
                d.rnd_info_id = Some(reader.read_u32_le()?);
                if d.rnd_info_id == Some(0) {
                    d.factions = Some(read_bitmask_factions(reader, ctx)?);
                }
                d.rnd_info_min_lev = Some(reader.read_u8()?);
                d.rnd_info_max_lev = Some(reader.read_u8()?);
            }
            RandomDwellingLvl(ref mut d) => {
                d.owner = Ownership::from(reader.read_u32_le()?);
                d.rnd_info_id = Some(reader.read_u32_le()?);
                if d.rnd_info_id == Some(0) {
                    d.factions = Some(read_bitmask_factions(reader, ctx)?);
                }
            }
            RandomDwellingFaction(ref mut d) => {
                d.owner = Ownership::from(reader.read_u32_le()?);
                d.rnd_info_min_lev = Some(reader.read_u8()?);
                d.rnd_info_max_lev = Some(reader.read_u8()?);
            }
            QuestGuard(ref mut m) => *m = read_quest(reader, ctx)?,
            HeroPlaceholder {
                ref mut owner,
                ref mut hero_id,
            } => {
                *owner = Ownership::from(reader.read_u8()? as u32);
                *hero_id = reader.read_u8()? as u32;
            }
            CreatureBank(ref mut b)
            | DerelictShip(ref mut b)
            | DragonUtopia(ref mut b)
            | Crypt(ref mut b)
            | Shipwreck(ref mut b) => {
                if ctx.level_HOTA3 {
                    b.guards_preset_index = reader.read_i32_le()?;
                    b.upgraded_stack_presence = reader.read_i8()?;
                    let artifacts_cnt = reader.read_u32_le()?;
                    for _ in 0..artifacts_cnt {
                        b.reward_artifacts
                            .push(read_artifact_id_from_i32(reader, ctx)?);
                    }
                }
            }
            _ => {}
        }
        ret.push(Object {
            position,
            obj_templ_id,
            obj_type,
        });
    }
    Ok(ret)
}

fn parse_events(reader: &mut BinaryDataReader, _ctx: &ParsingContext) -> io::Result<Vec<Event>> {
    let mut ret = Vec::new();
    Ok(ret)
}

fn read_box_content(reader: &mut BinaryDataReader, ctx: &ParsingContext) -> io::Result<BoxContent> {
    let guards = read_message_and_guards(reader, ctx)?;
    let reward_experience = reader.read_u32_le()?;
    let reward_mana_diff = reader.read_i32_le()?;
    let reward_next_battle_morale = reader.read_i8()?;
    let reward_next_battle_luck = reader.read_i8()?;
    let reward_resources = read_resource_pack(reader, ctx)?;
    let reward_primary_skills = PrimarySkills {
        attack: reader.read_u8()? as u32,
        defence: reader.read_u8()? as u32,
        spell_power: reader.read_u8()? as u32,
        knowledge: reader.read_u8()? as u32,
    };
    let mut reward_secondary_skills = Vec::new();
    for _ in 0..reader.read_u8()? {
        reward_secondary_skills.push(read_secondary_skill(reader, ctx)?);
    }
    let mut reward_artifacts = Vec::new();
    for _ in 0..reader.read_u8()? {
        if let Some(a) = read_artifact_id(reader, ctx)? {
            reward_artifacts.push(a);
        }
    }
    let mut reward_spells = Vec::new();
    for _ in 0..reader.read_u8()? {
        if let Some(s) = read_spell_id(reader, ctx)? {
            reward_spells.push(s);
        }
    }
    let mut reward_creatures = Vec::new();
    for _ in 0..reader.read_u8()? {
        if let Some(creature) = read_creature(reader, ctx)? {
            let amount = reader.read_u16_le()?;
            reward_creatures.push((creature, amount as u32));
        } else {
            let _ = reader.read_u16_le()?;
        }
    }
    reader.skip_n(8);
    Ok(BoxContent {
        guards,
        reward_experience,
        reward_mana_diff,
        reward_next_battle_morale,
        reward_next_battle_luck,
        reward_resources,
        reward_primary_skills,
        reward_secondary_skills,
        reward_artifacts,
        reward_spells,
        reward_creatures,
    })
}

fn read_message_and_guards(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Option<CreatureGuard>> {
    if reader.read_bool()? {
        let message = reader.read_string_le()?;
        let slot = if reader.read_bool()? {
            read_creature_set(reader, ctx)?
        } else {
            Vec::new()
        };
        reader.skip_n(4);
        Ok(Some(CreatureGuard { message, slot }))
    } else {
        Ok(None)
    }
}

fn read_creature_set(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Vec<CreatureSlot>> {
    const CREATURE_SET_SLOT: usize = 7;
    let mut ret = Vec::new();
    for slot_num in 0..CREATURE_SET_SLOT {
        let creature = read_creature(reader, ctx)?;
        let amount = reader.read_u16_le()? as u32;
        ret.push(CreatureSlot {
            slot_num: slot_num as u8,
            creature,
            amount,
        });
    }
    Ok(ret)
}

fn read_creature(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Option<CreatureId>> {
    let id = if ctx.level_AB {
        reader.read_u16_le()?
    } else {
        reader.read_u8()? as u16
    };
    if id == ctx.creature_identifier_invalid {
        Ok(None)
    } else {
        Ok(Some(CreatureId(id)))
    }
}

fn read_artifact_id(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Option<ArtifactId>> {
    if ctx.level_AB {
        let id = reader.read_u16_le()? as u32;
        if id == ctx.artifact_identifier_invalid {
            return Ok(None);
        }
        Ok(Some(ArtifactId(id)))
    } else {
        let id = reader.read_u8()? as u32;
        if id == ctx.artifact_identifier_invalid {
            return Ok(None);
        }
        Ok(Some(ArtifactId(id)))
    }
}

fn read_artifact_id_from_i32(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Option<ArtifactId>> {
    let id = reader.read_i32_le()?;
    if id as u32 == ctx.artifact_identifier_invalid {
        return Ok(None);
    }
    if id == -1 {
        return Ok(None);
    }
    Ok(Some(ArtifactId(id as u32)))
}

fn read_spell_id(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Option<SpellId>> {
    let id = reader.read_u8()?;
    if id == ctx.spell_identifier_invalid {
        return Ok(None);
    }
    Ok(Some(SpellId(id as u32)))
}

fn read_resource_pack(
    reader: &mut BinaryDataReader,
    _ctc: &ParsingContext,
) -> io::Result<ResourcePack> {
    let mut rpack = [0; 7];
    for r in rpack.iter_mut() {
        *r = reader.read_u32_le()?;
    }
    Ok(ResourcePack(rpack))
}

fn read_secondary_skill(
    reader: &mut BinaryDataReader,
    _ctx: &ParsingContext,
) -> io::Result<SecSkill> {
    let id = reader.read_u8()? as u32;
    let level_id = reader.read_u8()?;
    let level = match SecSkillLevel::from(level_id) {
        Some(l) => l,
        None => {
            return Err(gen_error(&format!(
                "parsing secondary skill error: got {level_id}"
            )))
        }
    };
    Ok(SecSkill { id, level })
}

fn read_heroes_artifacts(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<(Vec<HeroesArtifact>, Vec<ArtifactId>)> {
    let artifacts = if reader.read_bool()? {
        let mut ret = Vec::new();
        for slot in 0..ctx.artifact_slots_count {
            let artifact_id = match read_artifact_id(reader, ctx)? {
                Some(id) => id,
                None => continue,
            };
            ret.push(HeroesArtifact {
                artifact_id,
                slot_id: slot as u32,
            })
        }
        ret
    } else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut artifacts_in_bag = Vec::new();
    let amount = reader.read_u16_le()?;
    for _ in 0..amount {
        let artifact_id = match read_artifact_id(reader, ctx)? {
            Some(id) => id,
            None => continue,
        };
        artifacts_in_bag.push(artifact_id);
    }
    Ok((artifacts, artifacts_in_bag))
}

fn read_seer_hut_quest(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<SeerHutData> {
    let mut mission = QuestMission::default();
    if ctx.level_AB {
        mission = read_quest(reader, ctx)?;
    } else {
        let art_id = read_artifact_id(reader, ctx)?;
        if let Some(art_id) = art_id {
            mission = QuestMission {
                mission_type: QuestMissionType::Artifact(vec![art_id]),
                ..Default::default()
            };
        }
    }
    let mut reward = SeerHutRewardType::default();
    if mission.mission_type != QuestMissionType::NoMission {
        use SeerHutRewardType::*;
        let type_id = reader.read_u8()?;
        reward = SeerHutRewardType::from(type_id);
        match &mut reward {
            Nothing => {}
            Experience(ref mut exp) => *exp = reader.read_u32_le()?,
            ManaPoints(ref mut mana) => *mana = reader.read_u32_le()?,
            Morale(ref mut morale) => *morale = reader.read_i8()?,
            Luck(ref mut luck) => *luck = reader.read_i8()?,
            Resources(ref mut r) => *r = (reader.read_u8()?, reader.read_u32_le()?),
            PrimarySkills(ref mut prim) => {
                *prim = crate::map_structs::PrimarySkills::from(
                    reader.read_u8()?,
                    reader.read_u8()? as u32,
                )
            }
            SecondarySkills(ref mut skills) => {
                let id = reader.read_u8()? as u32;
                if let Some(level) = SecSkillLevel::from(reader.read_u8()?) {
                    skills.push(SecSkill { id, level });
                }
            }
            Artifact(ref mut vec) => {
                if let Some(a) = read_artifact_id(reader, ctx)? {
                    vec.push(a);
                }
            }
            Spell(ref mut vec) => vec.push(reader.read_u8()?),
            Creature(ref mut vec) => {
                if let Some(id) = read_creature(reader, ctx)? {
                    let amount = reader.read_u16_le()? as u32;
                    vec.push((id, amount));
                } else {
                    let _ = reader.read_u16_le()?;
                }
            }
        }
    } else {
        reader.skip_n(1);
    }
    Ok(SeerHutData {
        mission,
        reward,
        ..Default::default()
    })
}

fn read_quest(reader: &mut BinaryDataReader, ctx: &ParsingContext) -> io::Result<QuestMission> {
    use QuestMissionType::*;
    let id = reader.read_u8()?;
    let mut mission_type = QuestMissionType::from(id);
    match &mut mission_type {
        NoMission => {
            return Ok(QuestMission::default());
        }
        ExpLevel(ref mut exp) => *exp = reader.read_u32_le()?,
        PrimarySkill(ref mut prim) => {
            *prim = PrimarySkills {
                attack: reader.read_u8()? as u32,
                defence: reader.read_u8()? as u32,
                spell_power: reader.read_u8()? as u32,
                knowledge: reader.read_u8()? as u32,
            }
        }
        KillHero(ref mut hero_id) => *hero_id = reader.read_u32_le()?,
        KillCreature(ref mut creat_id) => *creat_id = reader.read_u32_le()?,
        Artifact(ref mut v) => {
            let amount = reader.read_u8()? as usize;
            for _ in 0..amount {
                if let Some(id) = read_artifact_id(reader, ctx)? {
                    v.push(id);
                }
            }
        }
        Army(ref mut a) => {
            let amount = reader.read_u8()?;
            for _ in 0..amount {
                if let Some(id) = read_creature(reader, ctx)? {
                    a.push((id, reader.read_u16_le()? as u32));
                } else {
                    let _ = reader.read_u16_le()?;
                }
            }
        }
        Resources(ref mut r) => {
            *r = read_resource_pack(reader, ctx)?;
            // println!("resources: {r:?}");
        }
        Hero(ref mut id) => *id = reader.read_u8()?,
        Player(ref mut pl) => {
            let id = reader.read_u8()? as usize;
            if id < ALL_PLAYERS.len() {
                *pl = Some(ALL_PLAYERS[id]);
            }
        }
        HOTAMulti => {
            let sub_mission = reader.read_u32_le()?;
            if sub_mission == 0 {
                let heroes_count = reader.read_u32_le()?;
                assert!(heroes_count < 256);
                let heroes = map_bits_to_numbers(reader, heroes_count as u8)?;
                mission_type = HOTAHeroClass(heroes);
            } else if sub_mission == 1 {
                mission_type = HOTAReachDate(reader.read_u32_le()?);
            }
        }
        Keymaster => {}
        HOTAHeroClass(_) => {}
        HOTAReachDate(_) => {}
    }
    let last_day = reader.read_i32_le()?;
    let proposal_message = reader.read_string_le()?;
    let progress_message = reader.read_string_le()?;
    let completion_message = reader.read_string_le()?;
    Ok(QuestMission {
        mission_type,
        last_day,
        proposal_message,
        progress_message,
        completion_message,
    })
}

fn read_bitmask_buildings(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Vec<Buildings>> {
    assert!(ctx.buildings_bytes < 256);
    Ok(map_bits_to_numbers(reader, ctx.buildings_count as u8)?
        .iter()
        .map(|code| Buildings::from(*code as i8 as i32))
        .collect::<Vec<_>>())
}

fn read_bitmask_factions(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Vec<Town>> {
    Ok(map_bits_to_objects(
        reader,
        &ctx.factions,
        ctx.factions_bytes,
    )?)
}

/// Read a byte and collect an item from `object`
/// if corresponding bit is set
fn map_bits_to_objects<T: Copy + Clone>(
    reader: &mut BinaryDataReader,
    objects: &[T],
    bytes_to_read: usize,
) -> io::Result<Vec<T>> {
    let mut ret = Vec::new();
    let mut mask = 0u8;
    let mut bytes_read = 0;
    for (i, o) in objects.iter().enumerate() {
        let bit_no = i % 8;
        if bit_no == 0 {
            mask = reader.read_u8()?;
            bytes_read += 1;
        }

        if mask & (1 << bit_no) != 0 {
            ret.push(*o);
        }
    }
    while bytes_read < bytes_to_read {
        let _ = reader.read_u8()?;
        bytes_read += 1;
    }
    if bytes_read > bytes_to_read {
        panic!(
            "More bytes have been read than specified: read={bytes_read}, expected={bytes_to_read}"
        );
    }
    Ok(ret)
}

/// E.g. reader.read_u8()->001100101 ==> [2, 3, 6, 8]
fn map_bits_to_numbers(reader: &mut BinaryDataReader, read_up_to: u8) -> io::Result<Vec<u8>> {
    let mut ret = Vec::new();
    let mut mask = 0u8;
    for i in 0..read_up_to {
        let bit_no = i % 8;
        if bit_no == 0 {
            mask = reader.read_u8()?;
        }
        if mask & (1 << bit_no) != 0 {
            ret.push(i);
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
    factions: Vec<Town>,
    factions_bytes: usize,
    heroes_bytes: usize,
    artifacts_bytes: usize,
    resources_bytes: usize,
    skills_bytes: usize,
    spells_bytes: usize,
    buildings_bytes: usize,

    // total number of elements of appropriate type
    factions_count: usize,
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
    hero_identifier_invalid: u8,
    artifact_identifier_invalid: u32,
    creature_identifier_invalid: u16,
    spell_identifier_invalid: u8,

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
        use Town::*;
        ctx.factions = vec![
            Castle, Rampart, Tower, Inferno, Necropolis, Dungeon, Stronghold, Fortress,
        ];

        ctx.factions_bytes = 1;
        ctx.heroes_bytes = 16;
        ctx.artifacts_bytes = 16;
        ctx.skills_bytes = 4;
        ctx.resources_bytes = 4;
        ctx.spells_bytes = 9;
        ctx.buildings_bytes = 6;

        ctx.factions_count = 8;
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

            ctx.factions.push(Conflux);
            ctx.factions_bytes = 2; // + Conflux
            ctx.factions_count = 9;

            ctx.creatures_count = 145; // + Conflux and new neutrals

            ctx.heroes_count = 156; // + Conflux and campaign heroes
            ctx.heroes_portraits_count = 159; // +_kendal, +young Cristian, +Ordwald
                                              // ctx.heroes_bytes = 20;

            ctx.artifacts_count = 129; // + Armaggedon Blade and Vial of Dragon Blood
                                       // ctx.artifacts_bytes = 17;

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
            ctx.factions.push(Cove);

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
