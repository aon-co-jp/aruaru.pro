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
];

#[cfg(test)]
mod tests {
    use super::*;

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
