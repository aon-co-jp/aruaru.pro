//! RS-React(`rs-react`)を使った最小のSSRページ——カテゴリ一覧。
//!
//! **現状**: 静的なカテゴリマスタ(`categories::CATEGORIES`)を
//! `App::mount`でコンポーネントとして初回render→`render_to_html`で
//! HTML文字列化するだけの、一方通行のSSR。`App::tick`による再レンダー
//! (状態変化への追従)はまだこのページでは使っていない——カテゴリ一覧
//! 自体が現時点でクライアント操作を持たないため。将来、絞り込み検索等の
//! インタラクティブなUIを追加する際に`App::tick`(サーバー側で状態を
//! 持たせる場合)またはWasmハイドレーション(クライアント側)を使う想定。

use rs_react::{App, VNode};

use crate::categories::CATEGORIES;

/// カテゴリ一覧ページのルートコンポーネントの`ComponentId`。
/// 現時点ではページ全体で1つだけマウントするため固定値で問題無い
/// (複数インスタンスを同時にマウントする必要が出た場合は、呼び出し側
/// が一意なidを割り当てる規約——`rs_react::hooks`のドキュメント参照)。
const CATEGORIES_PAGE_ROOT_ID: u64 = 1;

pub fn render_categories_page() -> String {
    let app = App::mount(CATEGORIES_PAGE_ROOT_ID, category_list_view);
    let body_html = rs_react::render_to_html(app.tree());
    format!(
        "<!DOCTYPE html><html lang=\"ja\"><head><meta charset=\"utf-8\">\
<title>aruaru.pro — カテゴリ一覧</title></head><body>{body_html}</body></html>"
    )
}

fn category_list_view() -> VNode {
    VNode::element("ul")
        .attr("id", "categories")
        .children(CATEGORIES.iter().map(category_item))
        .build()
}

fn category_item(category: &crate::categories::Category) -> VNode {
    if category.children.is_empty() {
        return VNode::element("li").child(VNode::text(category.name)).build();
    }
    VNode::element("li")
        .child(VNode::text(category.name))
        .child(
            VNode::element("ul")
                .children(category.children.iter().map(|child| VNode::element("li").child(VNode::text(*child)).build()))
                .build(),
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_all_top_level_categories_as_list_items() {
        let html = render_categories_page();
        assert!(html.contains("<ul id=\"categories\">"));
        for category in CATEGORIES {
            assert!(html.contains(category.name), "missing category: {}", category.name);
        }
    }

    #[test]
    fn design_category_subcategories_are_nested_in_their_own_list() {
        let html = render_categories_page();
        assert!(html.contains("ロゴ作成・ロゴデザイン"), "sub-category must appear in the rendered HTML");
    }
}
