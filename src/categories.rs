//! カテゴリマスタ(初期データ)。
//!
//! ユーザーが提示したcoconala(https://coconala.com/)の機能LISTを、
//! 初期カテゴリマスタとしてそのまま流用する(推測で埋めていない
//! ——ユーザー提示のリストに基づく)。デザイン制作は子カテゴリを持つ
//! (提示リストの通り)。

#[derive(Debug, Clone, serde::Serialize)]
pub struct Category {
    pub name: &'static str,
    pub children: &'static [&'static str],
}

pub const CATEGORIES: &[Category] = &[
    Category { name: "イラスト作成・漫画制作", children: &[] },
    Category {
        name: "デザイン制作",
        children: &[
            "ロゴ作成・ロゴデザイン",
            "チラシ作成・フライヤーデザイン",
            "建築・パース作成・インテリア",
            "名刺作成・名刺デザイン",
            "パンフレット作成・カタログデザイン",
            "ポスター作成・看板デザイン",
            "製品設計・プロダクトデザイン",
            "パッケージデザイン・ラベル作成",
            "ファッション・グッズデザイン",
            "結婚式アイテム作成・記念日デザイン",
            "書籍デザイン・表紙作成",
            "メニュー表作成・POPデザイン",
            "デザイン修正・サイズ変更",
            "写真加工・画像編集",
            "文字デザイン・筆文字作成",
            "展示会ブースデザイン作成",
            "AI生成画像の加工・レタッチ",
            "その他（デザイン制作）",
            "Webサイトデザイン",
            "サムネイル・画像デザイン",
            "デザインレッスン",
        ],
    },
    Category { name: "Web制作・HP作成・EC構築", children: &[] },
    Category { name: "動画編集・映像制作", children: &[] },
    Category { name: "集客・マーケティング相談", children: &[] },
    Category { name: "ビジネス代行・事務代行", children: &[] },
    Category { name: "音楽制作・ナレーション", children: &[] },
    Category { name: "IT相談・システム開発", children: &[] },
    Category { name: "ライティング・翻訳", children: &[] },
    Category { name: "コンサルティング・士業", children: &[] },
    Category { name: "生成AI活用・開発・制作", children: &[] },
    Category { name: "占い", children: &[] },
    Category { name: "悩み相談・カウンセリング", children: &[] },
    Category { name: "学習指導・資格・キャリア相談", children: &[] },
    Category { name: "住まい・美容・生活相談", children: &[] },
    Category { name: "オンラインレッスン・習い事", children: &[] },
    Category { name: "ハンドメイド制作", children: &[] },
    Category { name: "出張撮影・出張サービス", children: &[] },
    Category { name: "資産運用・副業の相談", children: &[] },
    Category { name: "弁護士検索・法律Q&A（法律相談）", children: &[] },
    // coconalaには無いが、今回の統合方針(コンコナラ+求人サイト)に
    // 基づき追加するトップレベルカテゴリ。
    Category { name: "求人・アルバイト情報", children: &[] },
    // ユーザー指示(2026-09-13)「正社員求人や無料のIT研修付き無料の
    // 転職エージェントサービスコーナーも作って」に基づき追加。
    // 正社員求人自体は上の「求人・アルバイト情報」
    // (`job_listings`テーブル、`employment_type = "正社員"`)で既に
    // カバーしているため、ここで新設するのは「無料IT研修+就職支援」
    // という異なるサービス形態(`career_agent_programs`テーブル)。
    Category { name: "IT研修付き転職エージェント", children: &[] },
];

/// DBテーブル化されたカテゴリツリーの1行(親カテゴリ、子カテゴリ名の
/// 配列)。`CATEGORIES`(コンパイル時定数、シードデータ兼オフライン用の
/// 高速パス)とは別に、DBへ永続化した実体をこの形へ再構築して返す
/// ——将来、管理者が再デプロイ無しでカテゴリを追加できるようにする
/// ための土台(`CLAUDE.md`の「次にすべきこと」参照)。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CategoryTreeRow {
    pub name: String,
    pub children: Vec<String>,
}

/// `CATEGORIES`(コンパイル時定数)を`CategoryTreeRow`列へ変換する。
/// DBに何も無い場合のフォールバック、およびテスト用の比較対象として使う。
pub fn static_category_tree() -> Vec<CategoryTreeRow> {
    CATEGORIES
        .iter()
        .map(|c| CategoryTreeRow {
            name: c.name.to_string(),
            children: c.children.iter().map(|s| s.to_string()).collect(),
        })
        .collect()
}

pub async fn ensure_table_and_seed(db: &aruaru_db_connector::AruaruDb) -> anyhow::Result<()> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS categories (name TEXT PRIMARY KEY, parent_name TEXT, position INTEGER)",
        &[],
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    for (top_position, cat) in CATEGORIES.iter().enumerate() {
        let top_position = top_position as i32;
        db.execute(
            "INSERT INTO categories (name, parent_name, position) VALUES ($1, NULL, $2) \
             ON CONFLICT (name) DO NOTHING",
            &[&cat.name, &top_position],
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

        for (child_position, child) in cat.children.iter().enumerate() {
            let child_position = child_position as i32;
            db.execute(
                "INSERT INTO categories (name, parent_name, position) VALUES ($1, $2, $3) \
                 ON CONFLICT (name) DO NOTHING",
                &[child, &cat.name, &child_position],
            )
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
    }
    Ok(())
}

/// `name`がDB上のカテゴリとして存在するかを確認する(コンパイル時定数に
/// 無い、後から追加されたカテゴリも含めて検証できる——出品作成時の
/// カテゴリ検証で、まずコンパイル時定数を高速に確認し、無ければこちらへ
/// フォールバックする使い方を想定)。
pub async fn category_exists_in_db(db: &aruaru_db_connector::AruaruDb, name: &str) -> Result<bool, String> {
    db.query("SELECT 1 FROM categories WHERE name = $1 LIMIT 1", &[&name])
        .await
        .map(|rows| !rows.is_empty())
        .map_err(|e| e.to_string())
}

/// DB上のカテゴリツリー(親子関係・表示順)を読み出す。`position`列で
/// ソートすることで、シード時の(=ユーザー提示リストの)並び順を保つ。
pub async fn load_category_tree(db: &aruaru_db_connector::AruaruDb) -> Result<Vec<CategoryTreeRow>, String> {
    let rows = db
        .query("SELECT name, parent_name FROM categories ORDER BY position", &[])
        .await
        .map_err(|e| e.to_string())?;

    let mut top: Vec<CategoryTreeRow> = Vec::new();
    let mut children_by_parent: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for row in &rows {
        let name: String = row.get(0);
        let parent_name: Option<String> = row.get(1);
        match parent_name {
            None => top.push(CategoryTreeRow { name, children: Vec::new() }),
            Some(parent) => children_by_parent.entry(parent).or_default().push(name),
        }
    }
    for cat in &mut top {
        if let Some(children) = children_by_parent.remove(&cat.name) {
            cat.children = children;
        }
    }
    Ok(top)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_category_tree_matches_categories_len_and_names() {
        let tree = static_category_tree();
        assert_eq!(tree.len(), CATEGORIES.len());
        for (row, cat) in tree.iter().zip(CATEGORIES.iter()) {
            assert_eq!(row.name, cat.name);
            assert_eq!(row.children.len(), cat.children.len());
        }
    }

    #[test]
    fn categories_are_non_empty_and_names_are_unique() {
        assert!(!CATEGORIES.is_empty());
        let mut names: Vec<&str> = CATEGORIES.iter().map(|c| c.name).collect();
        let len_before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), len_before, "category names must be unique");
    }

    #[test]
    fn design_category_carries_its_coconala_subcategories() {
        let design = CATEGORIES.iter().find(|c| c.name == "デザイン制作").expect("デザイン制作 must exist");
        assert!(design.children.contains(&"ロゴ作成・ロゴデザイン"));
        assert!(design.children.len() >= 20);
    }
}
