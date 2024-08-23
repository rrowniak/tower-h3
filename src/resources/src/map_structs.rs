use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    ROE, // 14
    AB,  // 21
    SOD, // 28
    //CHR   = 0x1d, // 29 Heroes Chronicles, presumably - identical to SoD, untested
    HOTA, // 32
    WOG,  // 51
    VCMI,
}

impl Format {
    pub fn from(code: u32) -> Option<Self> {
        match code {
            0x0e => Some(Self::ROE),
            0x15 => Some(Self::AB),
            0x1c => Some(Self::SOD),
            0x20 => Some(Self::HOTA),
            0x33 => Some(Self::WOG),
            0x64 => Some(Self::VCMI),
            _ => None,
        }
    }

    pub fn nice_str(&self) -> &'static str {
        match *self {
            Self::ROE => "Restoration of Erathia",
            Self::AB => "Armageddon blade",
            Self::SOD => "Shadof of death",
            Self::HOTA => "Horn of the abbys",
            Self::WOG => "Wake of gods",
            Self::VCMI => "VCMI",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
    Expert,
    Impossible,
}

impl Difficulty {
    pub fn from(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Easy),
            1 => Some(Self::Normal),
            2 => Some(Self::Hard),
            3 => Some(Self::Expert),
            4 => Some(Self::Impossible),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Player {
    #[default]
    Red,
    Blue,
    Tan,
    Green,
    Orange,
    Purple,
    Teal,
    Pink,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum PlayerBehaviour {
    #[default]
    None,
    Random,
    Warrior,
    Builder,
    Explorer,
}

impl PlayerBehaviour {
    pub fn from(code: i8) -> Option<Self> {
        match code {
            -1 => Some(Self::None),
            0 => Some(Self::Random),
            1 => Some(Self::Warrior),
            2 => Some(Self::Builder),
            3 => Some(Self::Explorer),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Town {
    Castle,
    Rampart,
    Tower,
    Inferno,
    Necropolis,
    Dungeon,
    Stronghold,
    Fortress,
    Conflux,
    Cove,
    Factory,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Faction {
    #[default]
    RandomAll,
    RandomSome(Vec<Town>),
    Some(Town),
    None,
}

#[derive(Debug)]
pub struct Hero {
    pub id: u8,
    pub portrait_id: Option<u8>,
    pub name: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MapCoord {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

impl MapCoord {
    pub fn from(x: u8, y: u8, z: u8) -> Self {
        Self {
            x: x as usize,
            y: y as usize,
            z: z as usize,
        }
    }
}

#[derive(Debug, Default)]
pub struct PlayerInfo {
    pub player: Player,
    pub can_be_human: bool,
    pub can_be_computer: bool,
    pub behaviour: PlayerBehaviour,
    pub faction: Faction,
    pub generate_hero_at_main_town: bool,
    pub main_town_position: Option<MapCoord>,
    pub has_random_hero: bool,
    pub lead_hero: Option<Hero>,
    pub other_heroes: Vec<Hero>,
}

#[derive(Debug)]
pub struct Info {
    pub format: Format,
    pub any_players: bool,
    pub map_dimension: i32,
    pub two_levels: bool,
    pub name: String,
    pub description: String,
    pub difficulty: Difficulty,
    pub hero_level_limit: Option<usize>,
}

#[derive(Debug)]
pub enum Resource {
    Wood,
    Mercury,
    Ore,
    Sulfur,
    Crystal,
    Gems,
    Gold,
}

impl Resource {
    pub fn from(code: u8) -> Option<Self> {
        use Resource::*;
        match code {
            0 => Some(Wood),
            1 => Some(Mercury),
            2 => Some(Ore),
            3 => Some(Sulfur),
            4 => Some(Crystal),
            5 => Some(Gems),
            6 => Some(Gold),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Building {
    Town,
    City,
    Capitol,
    Fort,
    Citadel,
    Castle,
}

#[derive(Debug)]
pub struct ArtifactId(pub u32);

#[derive(Debug)]
pub enum SpecialVictoryCondition {
    AcquireArtifact {
        artifact_code: ArtifactId,
    },
    AccumulateCreatures {
        unit_code: u16,
        amount: u32,
    },
    AccumulateResources {
        resource: Resource,
        amount: u32,
    },
    UpgradeTown {
        town_coord: MapCoord,
        hall_level: Building,
        castle_level: Building,
    },
    BuildGrail {
        town_coord: MapCoord,
    },
    DefeatHero {
        hero_coord: MapCoord,
    },
    CaptureTown {
        town_coord: MapCoord,
    },
    DefeatMonster {
        monster_coord: MapCoord,
    },
    FlagAllCreatureDwellings,
    FlagAllMines,
    TransportArtifact {
        artifact_code: ArtifactId,
        artifact_coord: MapCoord,
    },
    // HOTA
    EliminateAllMonsters,
    //HOTA
    SurviveNDays {
        limit_days: u32,
    },
}

#[derive(Debug)]
pub enum SpecialLossCondition {
    LossTown { town_coord: MapCoord },
    LossHero { hero_coord: MapCoord },
    TimeExpires { limit_days: u32 },
}

#[derive(Debug)]
pub struct WinLossCond {
    pub allow_normal_victory: bool,
    pub victory_cond_applies_to_comp: bool,
    pub special_victory_cond: Option<SpecialVictoryCondition>,
    pub special_loss_cond: Option<SpecialLossCondition>,
}

#[derive(Debug)]
pub struct TeamInfo {
    pub teams: HashMap<u8, Vec<Player>>,
}

impl TeamInfo {
    pub fn new() -> Self {
        Self {
            teams: HashMap::new(),
        }
    }
    pub fn add(&mut self, team_num: u8, player: Player) {
        if let Some(players) = self.teams.get_mut(&team_num) {
            players.push(player);
        } else {
            self.teams.insert(team_num, vec![player]);
        }
    }
}

#[derive(Debug)]
pub struct HeroesDef {
    pub allowed_heroes: Vec<u8>,
    pub reserved_for_campaign: Vec<u8>,
    pub disposed_heroes: Vec<(Hero, Vec<Player>)>,
}

#[derive(Debug)]
pub struct MapOptions {
    pub allow_special_months: bool,
    pub round_limit: Option<u32>,
}

#[derive(Debug)]
pub struct AllowedArtifacts {
    pub artifacts: Vec<ArtifactId>,
}

#[derive(Debug)]
pub struct AllowedSpells {
    pub spells: Vec<u8>,
    pub skills: Vec<u8>,
}

#[derive(Debug)]
pub struct Rumor {
    pub name: String,
    pub rumor: String,
}

#[derive(Debug)]
pub struct Rumors {
    // vector with rumor names
    pub rumors: Vec<Rumor>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SecSkillLevel {
    Basic,
    Advanced,
    Expert,
}

impl SecSkillLevel {
    pub fn from(code: u8) -> Option<Self> {
        use SecSkillLevel::*;
        match code {
            0 => Some(Basic),
            1 => Some(Advanced),
            2 => Some(Expert),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct SecSkill {
    pub id: u32,
    pub level: SecSkillLevel,
}

#[derive(Debug)]
pub struct HeroesArtifact {
    pub artifact_id: ArtifactId,
    pub slot_id: u32,
}

#[derive(Debug)]
pub enum Gender {
    Male,
    Female,
}

#[derive(Debug)]
pub struct PrimarySkills {
    pub attack: u32,
    pub defence: u32,
    pub spell_power: u32,
    pub knowledge: u32,
}

#[derive(Debug)]
pub struct PredefinedHero {
    pub id: u32,
    pub experience: u32,
    pub secondary_skills: Vec<SecSkill>,
    pub artifacts: Vec<HeroesArtifact>,
    pub artifacts_in_bag: Vec<ArtifactId>,
    pub custom_bio: Option<String>,
    pub gender: Option<Gender>,
    pub custom_spells: Vec<u8>,
    pub custom_primary_skills: Option<PrimarySkills>,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum Surface {
    Dirt,
    Sand,
    #[default]
    Grass,
    Snow,
    Swamp,
    Rough,
    Subterranean,
    Lava,
    Water,
    Rock,
}

impl Surface {
    pub fn from(code: u8) -> Option<Self> {
        use Surface::*;
        match code {
            0 => Some(Dirt),
            1 => Some(Sand),
            2 => Some(Grass),
            3 => Some(Snow),
            4 => Some(Swamp),
            5 => Some(Rough),
            6 => Some(Subterranean),
            7 => Some(Lava),
            8 => Some(Water),
            9 => Some(Rock),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum RiverType {
    #[default]
    Clear,
    Icy,
    Muddy,
    Lava,
}

impl RiverType {
    pub fn from(code: u8) -> Option<Self> {
        use RiverType::*;
        match code {
            1 => Some(Clear),
            2 => Some(Icy),
            3 => Some(Muddy),
            4 => Some(Lava),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum RoadType {
    #[default]
    Dirt,
    Gravel,
    Cobblestone,
}

impl RoadType {
    pub fn from(code: u8) -> Option<Self> {
        use RoadType::*;
        match code {
            1 => Some(Dirt),
            2 => Some(Gravel),
            3 => Some(Cobblestone),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TerrainTile {
    pub surface_type: Surface,
    pub surface_picture: u8,
    pub river_type: Option<RiverType>,
    pub river_direction: u8,
    pub road_type: Option<RoadType>,
    pub road_direction: u8,
    pub mirroring_flags: u8,
}

pub type Terrain = Vec<Vec<TerrainTile>>;

#[derive(Debug, Default, Copy, Clone)]
pub enum TileTransitProperty {
    #[default]
    Transitable,
    TransitBlocked,
    Visitable,
}

#[derive(Debug)]
pub enum ObjectKind {
    Unknown(u8),
    Town,
    Monster,
    Hero,
    Artifact,
    Resource,
}

impl ObjectKind {
    pub fn from(code: u8) -> Self {
        use ObjectKind::*;
        match code {
            1 => Town,
            2 => Monster,
            3 => Hero,
            4 => Artifact,
            5 => Resource,
            u => Unknown(u),
        }
    }
}

#[derive(Debug)]
pub struct ObjectTemplate {
    pub animation_file: String,
    pub transit_matrix: [[TileTransitProperty; 8]; 6],
    pub allowed_terrains: Vec<Surface>,
    pub id: u32,
    pub subid: u32,
    pub obj_kind: ObjectKind,
    pub render_priority: u8,
}

use crate::map_obj_type::ObjectType;

#[derive(Debug)]
pub struct Object {
    pub position: MapCoord,
    pub obj_templ_id: u32,
    pub obj_type: ObjectType,
}

#[derive(Debug)]
pub struct ResourcePack(pub [u32; 7]);

#[derive(Debug)]
pub struct Event {}

#[derive(Debug)]
pub struct Map {
    pub info: Info,
    pub players: Vec<PlayerInfo>,
    pub win_loss_cond: WinLossCond,
    pub team_info: TeamInfo,
    pub heroes_def: HeroesDef,
    pub map_options: MapOptions,
    pub allowed_artifacts: AllowedArtifacts,
    pub allowed_spells: AllowedSpells,
    pub rumors: Rumors,
    pub predefined_heroes: Vec<PredefinedHero>,
    pub terrains: Vec<Terrain>,
    pub object_templates: Vec<ObjectTemplate>,
    pub objects: Vec<Object>,
    pub events: Vec<Event>,
}
