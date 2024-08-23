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
            main_town_position = Some(read_coord(reader)?);
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
        use Player::*;
        for player in [Red, Blue, Tan, Green, Orange, Purple, Teal, Pink] {
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
            use Player::*;
            let players =
                map_bits_to_objects(reader, &[Red, Blue, Tan, Green, Orange, Purple, Teal, Pink])?;
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
                let id = reader.read_u8()? as u32;
                let level = match SecSkillLevel::from(reader.read_u8()?) {
                    Some(l) => l,
                    None => return Err(gen_error("parsing secondary skill error")),
                };
                ret.push(SecSkill { id, level });
            }
            ret
        } else {
            Vec::new()
        };
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
            Vec::new()
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
    use crate::map_obj_type::ObjectType;
    for _ in 0..amount {
        reader.dump_hex(0, 16 * 4)?;
        let position = read_coord(reader)?;
        let obj_templ_id = reader.read_u32_le()?;
        reader.skip_n(5);
        let obj_id = templates[obj_templ_id as usize].id;
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
                println!("amount = {}", m.amount);
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
                println!("{m:?}");
            }

            Event(ref mut ev) => {}
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

fn read_artifact_id(
    reader: &mut BinaryDataReader,
    ctx: &ParsingContext,
) -> io::Result<Option<ArtifactId>> {
    if ctx.level_AB {
        let id = reader.read_u16_le()? as u32;
        if id == 0xffff {
            return Ok(None);
        }
        Ok(Some(ArtifactId(id)))
    } else {
        let id = reader.read_u8()? as u32;
        if id == 0xff {
            return Ok(None);
        }
        Ok(Some(ArtifactId(id)))
    }
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
    // factions_bytes: usize,
    // heroes_bytes: usize,
    // artifacts_bytes: usize,
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
        // ctx.heroes_bytes = 16;
        // ctx.artifacts_bytes = 16;
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
                                       // ctx.artifacts_bytes = 18;

            ctx.heroes_portraits_count = 163; // +Finneas +young Gem +young Sandro +young Yog

            ctx.artifact_slots_count = 19; // + MISC_5 slot
        }

        // HOTA
        if [Format::HOTA].contains(&map_format) {
            ctx.level_HOTA0 = true;
            ctx.level_HOTA1 = hota_version > 0;
            //ctxresult.levelHOTA2 = hotaVersion > 1; // HOTA2 seems to be identical to HOTA1 so far
            ctx.level_HOTA3 = hota_version > 2;

            // ctx.artifacts_bytes = 21;
            // ctx.heroes_bytes = 23;

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
