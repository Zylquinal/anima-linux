use rusqlite::{Connection, Result};
use std::path::PathBuf;
use directories::ProjectDirs;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    pub id: i32,
    pub name: String,
    pub file_path: String,
    pub base_opacity: f64,
    pub scale: f64,
    pub auto_spawn: bool,
}

#[derive(Debug, Clone)]
pub struct InstanceConfig {
    pub id: i32,
    pub animation_id: i32,
    pub scale: f64,
    pub opacity: f64,
    pub x: i32,
    pub y: i32,
    pub auto_spawn: bool,
    pub mirror: bool,
    pub flip_v: bool,
    pub roll: f64,
    pub pitch: f64,
    pub yaw: f64,
    pub temperature: f64, // -100 to 100
    pub contrast: f64,    // -100 to 100
    pub brightness: f64,  // -100 to 100
    pub saturation: f64,  // -100 to 100
    pub hue: f64,         // -180 to 180
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path();
        let conn = Connection::open(db_path)?;
        let mut db = Self { conn };
        db.init()?;
        db.migrate()?;
        Ok(db)
    }

    pub fn app_dir() -> PathBuf {
        if let Ok(config_path) = std::env::var("ANIMA_CONFIG") {
            let dir = PathBuf::from(config_path);
            std::fs::create_dir_all(&dir).unwrap_or(());
            return dir;
        }
        let proj_dirs = ProjectDirs::from("com", "github", "anima-linux")
            .expect("Failed to get project directories");
        let dir = proj_dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&dir).unwrap_or(());
        dir
    }

    fn get_db_path() -> PathBuf {
        Self::app_dir().join("anima.db")
    }

    fn init(&mut self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS animations (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                base_opacity REAL NOT NULL DEFAULT 1.0,
                scale REAL NOT NULL DEFAULT 1.0,
                auto_spawn INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS instances (
                id INTEGER PRIMARY KEY,
                animation_id INTEGER NOT NULL,
                scale REAL NOT NULL DEFAULT 1.0,
                opacity REAL NOT NULL DEFAULT 1.0,
                x INTEGER NOT NULL DEFAULT 0,
                y INTEGER NOT NULL DEFAULT 0,
                auto_spawn INTEGER NOT NULL DEFAULT 0,
                mirror INTEGER NOT NULL DEFAULT 0,
                flip_v INTEGER NOT NULL DEFAULT 0,
                roll REAL NOT NULL DEFAULT 0.0,
                pitch REAL NOT NULL DEFAULT 0.0,
                yaw REAL NOT NULL DEFAULT 0.0,
                temperature REAL NOT NULL DEFAULT 0.0,
                contrast REAL NOT NULL DEFAULT 0.0,
                brightness REAL NOT NULL DEFAULT 0.0,
                saturation REAL NOT NULL DEFAULT 0.0,
                hue REAL NOT NULL DEFAULT 0.0,
                FOREIGN KEY(animation_id) REFERENCES animations(id) ON DELETE CASCADE
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('max_spawns', '10')",
            [],
        )?;
        
        self.conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('live_update_delay', '300')",
            [],
        )?;
        
        self.conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('live_update_enabled', '1')",
            [],
        )?;

        self.conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('gnome_always_on_top_key', '<Control><Super>t')",
            [],
        )?;

        Ok(())
    }

    fn migrate(&mut self) -> Result<()> {
        let columns = vec![
            ("mirror", "INTEGER NOT NULL DEFAULT 0"),
            ("flip_v", "INTEGER NOT NULL DEFAULT 0"),
            ("roll", "REAL NOT NULL DEFAULT 0.0"),
            ("pitch", "REAL NOT NULL DEFAULT 0.0"),
            ("yaw", "REAL NOT NULL DEFAULT 0.0"),
            ("temperature", "REAL NOT NULL DEFAULT 0.0"),
            ("contrast", "REAL NOT NULL DEFAULT 0.0"),
            ("brightness", "REAL NOT NULL DEFAULT 0.0"),
            ("saturation", "REAL NOT NULL DEFAULT 0.0"),
            ("hue", "REAL NOT NULL DEFAULT 0.0"),
        ];

        for (name, def) in columns {
            let exists: bool = self.conn.query_row(
                &format!("SELECT count(*) FROM pragma_table_info('instances') WHERE name='{}'", name),
                [],
                |row| row.get(0),
            )?;
            if !exists {
                self.conn.execute(&format!("ALTER TABLE instances ADD COLUMN {} {}", name, def), [])?;
            }
        }
        Ok(())
    }

    pub fn get_max_spawns(&self) -> Result<i32> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = 'max_spawns'")?;
        let str_val: String = stmt.query_row([], |row| row.get(0))?;
        Ok(str_val.parse().unwrap_or(10))
    }

    pub fn set_max_spawns(&self, limit: i32) -> Result<()> {
        self.conn.execute(
            "REPLACE INTO settings (key, value) VALUES ('max_spawns', ?1)",
            [limit.to_string()],
        )?;
        Ok(())
    }

    pub fn get_live_update_delay(&self) -> Result<u64> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = 'live_update_delay'")?;
        let str_val: String = stmt.query_row([], |row| row.get(0)).unwrap_or_else(|_| "300".to_string());
        Ok(str_val.parse().unwrap_or(300))
    }

    pub fn set_live_update_delay(&self, delay: u64) -> Result<()> {
        self.conn.execute(
            "REPLACE INTO settings (key, value) VALUES ('live_update_delay', ?1)",
            [delay.to_string()],
        )?;
        Ok(())
    }

    pub fn get_live_update_enabled(&self) -> Result<bool> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = 'live_update_enabled'")?;
        let str_val: String = stmt.query_row([], |row| row.get(0)).unwrap_or_else(|_| "1".to_string());
        Ok(str_val == "1")
    }

    pub fn set_live_update_enabled(&self, enabled: bool) -> Result<()> {
        self.conn.execute(
            "REPLACE INTO settings (key, value) VALUES ('live_update_enabled', ?1)",
            [if enabled { "1" } else { "0" }],
        )?;
        Ok(())
    }

    pub fn get_gnome_always_on_top_key(&self) -> Result<String> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = 'gnome_always_on_top_key'")?;
        stmt.query_row([], |row| row.get(0))
            .or(Ok("<Control><Super>t".to_string()))
    }

    pub fn set_gnome_always_on_top_key(&self, key: &str) -> Result<()> {
        self.conn.execute(
            "REPLACE INTO settings (key, value) VALUES ('gnome_always_on_top_key', ?1)",
            [key],
        )?;
        Ok(())
    }


    pub fn get_all_animations(&self) -> Result<Vec<AnimationConfig>> {
        let mut stmt = self.conn.prepare("SELECT id, name, file_path, base_opacity, scale, auto_spawn FROM animations")?;
        let rows = stmt.query_map([], |row| {
            Ok(AnimationConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                file_path: row.get(2)?,
                base_opacity: row.get(3)?,
                scale: row.get(4)?,
                auto_spawn: row.get::<_, i32>(5)? == 1,
            })
        })?;

        let mut anims = Vec::new();
        for r in rows {
            anims.push(r?);
        }
        Ok(anims)
    }

    pub fn get_all_instances(&self) -> Result<Vec<InstanceConfig>> {
        let mut stmt = self.conn.prepare("SELECT id, animation_id, scale, opacity, x, y, auto_spawn, mirror, temperature, contrast, brightness, saturation, hue, flip_v, roll, pitch, yaw FROM instances")?;
        let rows = stmt.query_map([], |row| {
            Ok(InstanceConfig {
                id: row.get(0)?,
                animation_id: row.get(1)?,
                scale: row.get(2)?,
                opacity: row.get(3)?,
                x: row.get(4)?,
                y: row.get(5)?,
                auto_spawn: row.get::<_, i32>(6)? == 1,
                mirror: row.get::<_, i32>(7)? == 1,
                temperature: row.get(8)?,
                contrast: row.get(9)?,
                brightness: row.get(10)?,
                saturation: row.get(11)?,
                hue: row.get(12)?,
                flip_v: row.get::<_, i32>(13)? == 1,
                roll: row.get(14)?,
                pitch: row.get(15)?,
                yaw: row.get(16)?,
            })
        })?;

        let mut instances = Vec::new();
        for r in rows {
            instances.push(r?);
        }
        Ok(instances)
    }

    pub fn insert_animation(&self, name: &str, file_path: &str) -> Result<i32> {
        self.conn.execute(
            "INSERT INTO animations (name, file_path, base_opacity, scale, auto_spawn) VALUES (?1, ?2, 1.0, 1.0, 0)",
            (name, file_path),
        )?;
        Ok(self.conn.last_insert_rowid() as i32)
    }

    pub fn insert_instance(&self, animation_id: i32, scale: f64, opacity: f64, x: i32, y: i32, auto_spawn: bool) -> Result<i32> {
        self.conn.execute(
            "INSERT INTO instances (animation_id, scale, opacity, x, y, auto_spawn, mirror, flip_v, roll, pitch, yaw, temperature, contrast, brightness, saturation, hue) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)",
            (animation_id, scale, opacity, x, y, if auto_spawn { 1 } else { 0 }),
        )?;
        Ok(self.conn.last_insert_rowid() as i32)
    }

    pub fn update_instance_auto_spawn(&self, id: i32, auto_spawn: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE instances SET auto_spawn = ?1 WHERE id = ?2",
            (if auto_spawn { 1 } else { 0 }, id),
        )?;
        Ok(())
    }

    pub fn update_instance_scale(&self, id: i32, scale: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE instances SET scale = ?1 WHERE id = ?2",
            (scale, id),
        )?;
        Ok(())
    }

    pub fn update_instance_position(&self, id: i32, x: i32, y: i32) -> Result<()> {
        self.conn.execute(
            "UPDATE instances SET x = ?1, y = ?2 WHERE id = ?3",
            (x, y, id),
        )?;
        Ok(())
    }

    pub fn update_instance_mirror(&self, id: i32, mirror: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE instances SET mirror = ?1 WHERE id = ?2",
            (if mirror { 1 } else { 0 }, id),
        )?;
        Ok(())
    }

    pub fn update_instance_editing(&self, id: i32, temp: f64, contrast: f64, brightness: f64, saturation: f64, hue: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE instances SET temperature = ?1, contrast = ?2, brightness = ?3, saturation = ?4, hue = ?5 WHERE id = ?6",
            (temp, contrast, brightness, saturation, hue, id),
        )?;
        Ok(())
    }

    pub fn update_instance_rotation(&self, id: i32, flip_v: bool, roll: f64, pitch: f64, yaw: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE instances SET flip_v = ?1, roll = ?2, pitch = ?3, yaw = ?4 WHERE id = ?5",
            (if flip_v { 1 } else { 0 }, roll, pitch, yaw, id),
        )?;
        Ok(())
    }

    pub fn update_instance_opacity(&self, id: i32, opacity: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE instances SET opacity = ?1 WHERE id = ?2",
            (opacity, id),
        )?;
        Ok(())
    }

    pub fn delete_instance(&self, id: i32) -> Result<()> {
        self.conn.execute("DELETE FROM instances WHERE id = ?1", [id])?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_animation_auto_spawn(&self, id: i32, auto_spawn: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE animations SET auto_spawn = ?1 WHERE id = ?2",
            (if auto_spawn { 1 } else { 0 }, id),
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_animation_opacity(&self, id: i32, opacity: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE animations SET base_opacity = ?1 WHERE id = ?2",
            (opacity, id),
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_animation_scale(&self, id: i32, scale: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE animations SET scale = ?1 WHERE id = ?2",
            (scale, id),
        )?;
        Ok(())
    }

    pub fn delete_animation(&self, id: i32) -> Result<()> {
        self.conn.execute("DELETE FROM animations WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn rename_animation(&self, id: i32, new_name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE animations SET name = ?1 WHERE id = ?2",
            (new_name, id),
        )?;
        Ok(())
    }

    pub fn clear_all_data(&self) -> Result<()> {
        self.conn.execute("DELETE FROM instances", [])?;
        self.conn.execute("DELETE FROM animations", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_insert_and_retrieve() {
        unsafe {
            std::env::set_var("ANIMA_CONFIG", "/tmp/anima_test_db");
        }
        let db = Db::new().unwrap();
        db.clear_all_data().unwrap();
        
        let anim_id = db.insert_animation("test", "test.gif").unwrap();
        let inst_id = db.insert_instance(anim_id, 1.0, 1.0, 0, 0, false).unwrap();
        
        let instances = db.get_all_instances().unwrap();
        assert_eq!(instances.len(), 1);
        let inst = &instances[0];
        assert_eq!(inst.id, inst_id);
        assert_eq!(inst.roll, 0.0);
        assert_eq!(inst.pitch, 0.0);
        assert_eq!(inst.yaw, 0.0);
        
        db.update_instance_rotation(inst_id, true, 45.0, 30.0, 15.0).unwrap();
        
        let instances_after = db.get_all_instances().unwrap();
        let inst_after = &instances_after[0];
        assert_eq!(inst_after.flip_v, true);
        assert_eq!(inst_after.roll, 45.0);
        assert_eq!(inst_after.pitch, 30.0);
        assert_eq!(inst_after.yaw, 15.0);
    }
}
