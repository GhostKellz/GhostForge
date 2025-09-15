use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use rusqlite::{Connection, params, OptionalExtension};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub executable: PathBuf,
    pub install_path: PathBuf,
    pub launcher: Option<String>,
    pub launcher_id: Option<String>,
    pub wine_version: Option<String>,
    pub wine_prefix: Option<PathBuf>,
    pub icon: Option<PathBuf>,
    pub banner: Option<PathBuf>,
    pub launch_arguments: Vec<String>,
    pub environment_variables: Vec<(String, String)>,
    pub pre_launch_script: Option<String>,
    pub post_launch_script: Option<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub playtime_minutes: u64,
    pub last_played: Option<DateTime<Utc>>,
    pub installed_date: DateTime<Utc>,
    pub favorite: bool,
    pub hidden: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    pub wine_settings: WineSettings,
    pub graphics_settings: GraphicsSettings,
    pub performance_settings: PerformanceSettings,
    pub compatibility_settings: CompatibilitySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WineSettings {
    pub wine_version: String,
    pub wine_arch: String, // win32 or win64
    pub windows_version: String, // win10, win7, winxp
    pub prefix_path: PathBuf,
    pub enable_dxvk: bool,
    pub enable_vkd3d: bool,
    pub enable_nvapi: bool,
    pub enable_esync: bool,
    pub enable_fsync: bool,
    pub dll_overrides: Vec<(String, String)>,
    pub winetricks_verbs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub resolution: Option<(u32, u32)>,
    pub fullscreen: bool,
    pub vsync: bool,
    pub fps_limit: Option<u32>,
    pub enable_mangohud: bool,
    pub enable_gamemode: bool,
    pub enable_gamescope: bool,
    pub gamescope_options: Option<String>,
    pub prime_run: bool, // NVIDIA Optimus
    pub dri_prime: Option<u8>, // AMD hybrid graphics
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    pub cpu_affinity: Option<Vec<u32>>,
    pub cpu_governor: Option<String>,
    pub nice_level: Option<i8>,
    pub ionice_class: Option<u8>,
    pub ionice_level: Option<u8>,
    pub memory_limit: Option<u64>,
    pub disable_compositor: bool,
    pub pulse_latency_msec: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilitySettings {
    pub protondb_rating: Option<String>,
    pub known_issues: Vec<String>,
    pub required_winetricks: Vec<String>,
    pub required_overrides: Vec<(String, String)>,
    pub launch_through_steam: bool,
    pub use_steam_runtime: bool,
    pub battleye_support: bool,
    pub eac_support: bool, // Easy Anti-Cheat
}

pub struct GameLibrary {
    connection: Connection,
}

impl GameLibrary {
    pub fn new(db_path: &PathBuf) -> Result<Self> {
        let connection = Connection::open(db_path)?;

        connection.execute(
            "CREATE TABLE IF NOT EXISTS games (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                executable TEXT NOT NULL,
                install_path TEXT NOT NULL,
                launcher TEXT,
                launcher_id TEXT,
                wine_version TEXT,
                wine_prefix TEXT,
                icon TEXT,
                banner TEXT,
                launch_arguments TEXT,
                environment_variables TEXT,
                pre_launch_script TEXT,
                post_launch_script TEXT,
                categories TEXT,
                tags TEXT,
                playtime_minutes INTEGER DEFAULT 0,
                last_played TEXT,
                installed_date TEXT NOT NULL,
                favorite INTEGER DEFAULT 0,
                hidden INTEGER DEFAULT 0,
                notes TEXT,
                settings TEXT
            )",
            [],
        )?;

        Ok(Self { connection })
    }

    pub fn add_game(&self, game: &Game) -> Result<()> {
        let launch_args = serde_json::to_string(&game.launch_arguments)?;
        let env_vars = serde_json::to_string(&game.environment_variables)?;
        let categories = serde_json::to_string(&game.categories)?;
        let tags = serde_json::to_string(&game.tags)?;

        self.connection.execute(
            "INSERT INTO games (
                id, name, executable, install_path, launcher, launcher_id,
                wine_version, wine_prefix, icon, banner, launch_arguments,
                environment_variables, pre_launch_script, post_launch_script,
                categories, tags, playtime_minutes, last_played, installed_date,
                favorite, hidden, notes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
            params![
                game.id,
                game.name,
                game.executable.to_str(),
                game.install_path.to_str(),
                game.launcher,
                game.launcher_id,
                game.wine_version,
                game.wine_prefix.as_ref().and_then(|p| p.to_str()),
                game.icon.as_ref().and_then(|p| p.to_str()),
                game.banner.as_ref().and_then(|p| p.to_str()),
                launch_args,
                env_vars,
                game.pre_launch_script,
                game.post_launch_script,
                categories,
                tags,
                game.playtime_minutes,
                game.last_played.map(|dt| dt.to_rfc3339()),
                game.installed_date.to_rfc3339(),
                game.favorite,
                game.hidden,
                game.notes,
            ],
        )?;

        Ok(())
    }

    pub fn get_game(&self, id: &str) -> Result<Option<Game>> {
        let mut stmt = self.connection.prepare(
            "SELECT * FROM games WHERE id = ?1"
        )?;

        let game = stmt.query_row([id], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                executable: PathBuf::from(row.get::<_, String>(2)?),
                install_path: PathBuf::from(row.get::<_, String>(3)?),
                launcher: row.get(4)?,
                launcher_id: row.get(5)?,
                wine_version: row.get(6)?,
                wine_prefix: row.get::<_, Option<String>>(7)?.map(PathBuf::from),
                icon: row.get::<_, Option<String>>(8)?.map(PathBuf::from),
                banner: row.get::<_, Option<String>>(9)?.map(PathBuf::from),
                launch_arguments: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
                environment_variables: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                pre_launch_script: row.get(12)?,
                post_launch_script: row.get(13)?,
                categories: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
                tags: serde_json::from_str(&row.get::<_, String>(15)?).unwrap_or_default(),
                playtime_minutes: row.get(16)?,
                last_played: row.get::<_, Option<String>>(17)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                installed_date: DateTime::parse_from_rfc3339(&row.get::<_, String>(18)?)
                    .unwrap()
                    .with_timezone(&Utc),
                favorite: row.get(19)?,
                hidden: row.get(20)?,
                notes: row.get(21)?,
            })
        }).optional()?;

        Ok(game)
    }

    pub fn list_games(&self) -> Result<Vec<Game>> {
        let mut stmt = self.connection.prepare("SELECT * FROM games WHERE hidden = 0")?;

        let games = stmt.query_map([], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                executable: PathBuf::from(row.get::<_, String>(2)?),
                install_path: PathBuf::from(row.get::<_, String>(3)?),
                launcher: row.get(4)?,
                launcher_id: row.get(5)?,
                wine_version: row.get(6)?,
                wine_prefix: row.get::<_, Option<String>>(7)?.map(PathBuf::from),
                icon: row.get::<_, Option<String>>(8)?.map(PathBuf::from),
                banner: row.get::<_, Option<String>>(9)?.map(PathBuf::from),
                launch_arguments: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
                environment_variables: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                pre_launch_script: row.get(12)?,
                post_launch_script: row.get(13)?,
                categories: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
                tags: serde_json::from_str(&row.get::<_, String>(15)?).unwrap_or_default(),
                playtime_minutes: row.get(16)?,
                last_played: row.get::<_, Option<String>>(17)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                installed_date: DateTime::parse_from_rfc3339(&row.get::<_, String>(18)?)
                    .unwrap()
                    .with_timezone(&Utc),
                favorite: row.get(19)?,
                hidden: row.get(20)?,
                notes: row.get(21)?,
            })
        })?;

        games.collect::<Result<Vec<_>, _>>().map_err(anyhow::Error::from)
    }

    pub fn update_game(&self, game: &Game) -> Result<()> {
        let launch_args = serde_json::to_string(&game.launch_arguments)?;
        let env_vars = serde_json::to_string(&game.environment_variables)?;
        let categories = serde_json::to_string(&game.categories)?;
        let tags = serde_json::to_string(&game.tags)?;

        self.connection.execute(
            "UPDATE games SET
                name = ?2,
                executable = ?3,
                install_path = ?4,
                launcher = ?5,
                launcher_id = ?6,
                wine_version = ?7,
                wine_prefix = ?8,
                icon = ?9,
                banner = ?10,
                launch_arguments = ?11,
                environment_variables = ?12,
                pre_launch_script = ?13,
                post_launch_script = ?14,
                categories = ?15,
                tags = ?16,
                playtime_minutes = ?17,
                last_played = ?18,
                favorite = ?19,
                hidden = ?20,
                notes = ?21
            WHERE id = ?1",
            params![
                game.id,
                game.name,
                game.executable.to_str(),
                game.install_path.to_str(),
                game.launcher,
                game.launcher_id,
                game.wine_version,
                game.wine_prefix.as_ref().and_then(|p| p.to_str()),
                game.icon.as_ref().and_then(|p| p.to_str()),
                game.banner.as_ref().and_then(|p| p.to_str()),
                launch_args,
                env_vars,
                game.pre_launch_script,
                game.post_launch_script,
                categories,
                tags,
                game.playtime_minutes,
                game.last_played.map(|dt| dt.to_rfc3339()),
                game.favorite,
                game.hidden,
                game.notes,
            ],
        )?;

        Ok(())
    }

    pub fn remove_game(&self, id: &str) -> Result<()> {
        self.connection.execute("DELETE FROM games WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn search_games(&self, query: &str) -> Result<Vec<Game>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.connection.prepare(
            "SELECT * FROM games WHERE
            (name LIKE ?1 OR tags LIKE ?1 OR categories LIKE ?1 OR notes LIKE ?1)
            AND hidden = 0"
        )?;

        let games = stmt.query_map([pattern], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                executable: PathBuf::from(row.get::<_, String>(2)?),
                install_path: PathBuf::from(row.get::<_, String>(3)?),
                launcher: row.get(4)?,
                launcher_id: row.get(5)?,
                wine_version: row.get(6)?,
                wine_prefix: row.get::<_, Option<String>>(7)?.map(PathBuf::from),
                icon: row.get::<_, Option<String>>(8)?.map(PathBuf::from),
                banner: row.get::<_, Option<String>>(9)?.map(PathBuf::from),
                launch_arguments: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
                environment_variables: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                pre_launch_script: row.get(12)?,
                post_launch_script: row.get(13)?,
                categories: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
                tags: serde_json::from_str(&row.get::<_, String>(15)?).unwrap_or_default(),
                playtime_minutes: row.get(16)?,
                last_played: row.get::<_, Option<String>>(17)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                installed_date: DateTime::parse_from_rfc3339(&row.get::<_, String>(18)?)
                    .unwrap()
                    .with_timezone(&Utc),
                favorite: row.get(19)?,
                hidden: row.get(20)?,
                notes: row.get(21)?,
            })
        })?;

        games.collect::<Result<Vec<_>, _>>().map_err(anyhow::Error::from)
    }

    pub fn delete_game(&self, id: &str) -> Result<()> {
        self.connection.execute("DELETE FROM games WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn update_playtime(&self, id: &str, additional_minutes: u64) -> Result<()> {
        self.connection.execute(
            "UPDATE games SET playtime_minutes = playtime_minutes + ?2, last_played = ?3 WHERE id = ?1",
            params![id, additional_minutes, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_games_by_launcher(&self, launcher: &str) -> Result<Vec<Game>> {
        let mut stmt = self.connection.prepare("SELECT * FROM games WHERE launcher = ?1 AND hidden = 0")?;

        let games = stmt.query_map([launcher], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                executable: PathBuf::from(row.get::<_, String>(2)?),
                install_path: PathBuf::from(row.get::<_, String>(3)?),
                launcher: row.get(4)?,
                launcher_id: row.get(5)?,
                wine_version: row.get(6)?,
                wine_prefix: row.get::<_, Option<String>>(7)?.map(PathBuf::from),
                icon: row.get::<_, Option<String>>(8)?.map(PathBuf::from),
                banner: row.get::<_, Option<String>>(9)?.map(PathBuf::from),
                launch_arguments: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
                environment_variables: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                pre_launch_script: row.get(12)?,
                post_launch_script: row.get(13)?,
                categories: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
                tags: serde_json::from_str(&row.get::<_, String>(15)?).unwrap_or_default(),
                playtime_minutes: row.get(16)?,
                last_played: row.get::<_, Option<String>>(17)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                installed_date: DateTime::parse_from_rfc3339(&row.get::<_, String>(18)?)
                    .unwrap()
                    .with_timezone(&Utc),
                favorite: row.get(19)?,
                hidden: row.get(20)?,
                notes: row.get(21)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(games)
    }

    pub fn get_favorites(&self) -> Result<Vec<Game>> {
        let mut stmt = self.connection.prepare("SELECT * FROM games WHERE favorite = 1 AND hidden = 0 ORDER BY last_played DESC")?;

        let games = stmt.query_map([], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                executable: PathBuf::from(row.get::<_, String>(2)?),
                install_path: PathBuf::from(row.get::<_, String>(3)?),
                launcher: row.get(4)?,
                launcher_id: row.get(5)?,
                wine_version: row.get(6)?,
                wine_prefix: row.get::<_, Option<String>>(7)?.map(PathBuf::from),
                icon: row.get::<_, Option<String>>(8)?.map(PathBuf::from),
                banner: row.get::<_, Option<String>>(9)?.map(PathBuf::from),
                launch_arguments: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
                environment_variables: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                pre_launch_script: row.get(12)?,
                post_launch_script: row.get(13)?,
                categories: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
                tags: serde_json::from_str(&row.get::<_, String>(15)?).unwrap_or_default(),
                playtime_minutes: row.get(16)?,
                last_played: row.get::<_, Option<String>>(17)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                installed_date: DateTime::parse_from_rfc3339(&row.get::<_, String>(18)?)
                    .unwrap()
                    .with_timezone(&Utc),
                favorite: row.get(19)?,
                hidden: row.get(20)?,
                notes: row.get(21)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(games)
    }

}