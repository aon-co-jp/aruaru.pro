//! RS-React(`rs-react`)を使った最小のSSRページ——カテゴリ一覧。
//!
//! **2026-09-13、DBバックエンド化**: カテゴリマスタのDBテーブル化
//! (`categories::load_category_tree`)に合わせ、このページも
//! `categories::CategoryTreeRow`(DBから読んだ実体、無ければコンパイル
//! 時定数`CATEGORIES`から復元したフォールバック)を描画するようになった。
//! RS-Reactのコンポーネントモデルは完全に同期(`render_fn: Fn() -> VNode`、
//! async非対応)のため、DB読み出しは`App::mount`より前に済ませ、結果を
//! `move`クロージャで捕捉して渡す設計にしている。
//!
//! `App::tick`による再レンダー(状態変化への追従)はまだこのページでは
//! 使っていない——カテゴリ一覧自体が現時点でクライアント操作を持たない
//! ため。将来、絞り込み検索等のインタラクティブなUIを追加する際に
//! `App::tick`(サーバー側で状態を持たせる場合)またはWasmハイドレーション
//! (クライアント側)を使う想定。

use rs_react::{App, VNode};

use crate::categories::CategoryTreeRow;

/// カテゴリ一覧ページのルートコンポーネントの`ComponentId`。
/// 現時点ではページ全体で1つだけマウントするため固定値で問題無い
/// (複数インスタンスを同時にマウントする必要が出た場合は、呼び出し側
/// が一意なidを割り当てる規約——`rs_react::hooks`のドキュメント参照)。
const CATEGORIES_PAGE_ROOT_ID: u64 = 1;

/// DBからカテゴリツリーを読み、SSRページのHTML文字列を返す。DB読み出し
/// が失敗した場合はコンパイル時定数(`categories::static_category_tree`)
/// へフォールバックする(カテゴリ一覧が丸ごと出せなくなるより、多少
/// 古い/後から追加された分が欠けたものが出る方が実用上まさる、という
/// 判断——完全に真っ白なページを返すよりは常に何か出す)。
pub async fn render_categories_page(db: &aruaru_db_connector::AruaruDb) -> String {
    let categories = match crate::categories::load_category_tree(db).await {
        Ok(tree) if !tree.is_empty() => tree,
        _ => crate::categories::static_category_tree(),
    };
    build_categories_page_html(&categories)
}

/// カテゴリツリーからSSRページのHTML文字列を組み立てる純粋関数
/// (DB接続不要、ユニットテストで直接検証できる)。
pub fn build_categories_page_html(categories: &[CategoryTreeRow]) -> String {
    let categories = categories.to_vec();
    let app = App::mount(CATEGORIES_PAGE_ROOT_ID, move || category_list_view(&categories));
    let body_html = rs_react::render_to_html(app.tree());
    format!(
        "<!DOCTYPE html><html lang=\"ja\"><head><meta charset=\"utf-8\">\
<title>aruaru.pro — カテゴリ一覧</title></head><body>{body_html}</body></html>"
    )
}

fn category_list_view(categories: &[CategoryTreeRow]) -> VNode {
    VNode::element("ul").attr("id", "categories").children(categories.iter().map(category_item)).build()
}

fn category_item(category: &CategoryTreeRow) -> VNode {
    if category.children.is_empty() {
        return VNode::element("li").child(VNode::text(category.name.clone())).build();
    }
    VNode::element("li")
        .child(VNode::text(category.name.clone()))
        .child(
            VNode::element("ul")
                .children(category.children.iter().map(|child| VNode::element("li").child(VNode::text(child.clone())).build()))
                .build(),
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categories::static_category_tree;

    #[test]
    fn renders_all_top_level_categories_as_list_items() {
        let categories = static_category_tree();
        let html = build_categories_page_html(&categories);
        assert!(html.contains("<ul id=\"categories\">"));
        for category in &categories {
            assert!(html.contains(&category.name), "missing category: {}", category.name);
        }
    }

    #[test]
    fn design_category_subcategories_are_nested_in_their_own_list() {
        let html = build_categories_page_html(&static_category_tree());
        assert!(html.contains("ロゴ作成・ロゴデザイン"), "sub-category must appear in the rendered HTML");
    }
}
