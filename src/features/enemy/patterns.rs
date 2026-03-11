// Matches: t_unit.csv
pub const ENEMY_STATS: &str = r"^t_unit\.csv$";

// Matches: enemy_icon_000.png
pub const ENEMY_ICON: &str = r"^enemy_icon_(\d{3})\.png$";

// Matches: 000_e.imgcut, 000_e.mamodel, 000_e.png
pub const ENEMY_ANIM_BASE: &str = r"^(\d{3})_e\.(imgcut|mamodel|png)$";

// Matches: 000_e00.maanim through 03, and 000_e_zombie00.maanim through 02
pub const ENEMY_MAANIM: &str = r"^(\d{3})_e(0[0-3]|_zombie0[0-2])\.maanim$";

// Matches: Enemyname_en.tsv
pub const ENEMY_NAME: &str = r"^Enemyname(?:_([a-z]{2}))?\.tsv$";

// Matches: EnemyPictureBook_en.csv, EnemyPictureBook2_en.csv, etc.
pub const ENEMY_PICTURE_BOOK: &str = r"^EnemyPictureBook(?:_([a-z]{2}))?\.csv$";
pub const ENEMY_PICTURE_BOOK_2: &str = r"^EnemyPictureBook2(?:_([a-z]{2}))?\.csv$";
pub const ENEMY_PICTURE_BOOK_QUESTION: &str = r"^EnemyPictureBookQuestion(?:_([a-z]{2}))?\.csv$";

// Matches: general enemy dictionary/exclude configs
pub const ENEMY_DICT_LIST: &str = r"^enemy_dictionary_list\.csv$";
pub const AUTOSET_EXCLUDE: &str = r"^autoset_exclude_enemy\.csv$";

// Matches: set_enemy001_zombie_back.maanim, etc.
pub const ENEMY_ZOMBIE_EFFECT: &str = r"^set_enemy001_zombie(?:_[a-z]+)?\.(imgcut|mamodel|png|maanim)$";