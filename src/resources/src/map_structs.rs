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

#[derive(Debug, Default)]
pub struct PlayerInfo {
    pub player: Player,
    pub can_be_human: bool,
    pub can_be_computer: bool,
    pub behaviour: PlayerBehaviour,
    pub faction: Faction,
    pub generate_hero_at_main_town: bool,
    pub main_town_position: Option<(usize, usize, usize)>,
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
pub struct Map {
    pub info: Info,
    pub players: Vec<PlayerInfo>,
}
