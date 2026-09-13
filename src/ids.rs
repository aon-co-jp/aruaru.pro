//! エンティティid生成の共通ヘルパー。
//!
//! **背景**: `job-site`のid生成(`slugify`した成分を`--`連結)を
//! そのまま転用すると、`slugify`(英数字以外を除去)が日本語の
//! 出品者名・タイトル等を軒並み空文字に潰し、異なる入力が同じidへ
//! 衝突する(このサービスは日本語が主言語のため、英語テストデータ前提の
//! job-siteの設計がそのまま当てはまらない)。ここでは各成分のハッシュを
//! 必ず付与して一意性を保証し、ASCII成分がある場合のみ読みやすいslugを
//! 前置する。
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// `parts`(各成分)のハッシュを必ず付与したidを生成する。`prefix`は
/// ASCII成分が1つも無い場合のフォールバック接頭辞(例: `"svc"`→
/// `"svc-<hash>"`)。
pub fn make_id(parts: &[&str], prefix: &str) -> String {
    let mut hasher = DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
        0u8.hash(&mut hasher); // 区切り: "ab"+"c" と "a"+"bc" のハッシュ衝突を避ける
    }
    let hash = hasher.finish();

    let slugs: Vec<String> = parts.iter().map(|p| slugify(p)).filter(|s| !s.is_empty()).collect();
    if slugs.is_empty() {
        format!("{prefix}-{hash:016x}")
    } else {
        format!("{}-{hash:016x}", slugs.join("--"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_japanese_inputs_do_not_collide() {
        let id_a = make_id(&["鈴木一郎", "ロゴ制作"], "svc");
        let id_b = make_id(&["佐藤花子", "ロゴ制作"], "svc");
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn same_input_is_deterministic() {
        let id_a = make_id(&["鈴木一郎", "ロゴ制作"], "svc");
        let id_b = make_id(&["鈴木一郎", "ロゴ制作"], "svc");
        assert_eq!(id_a, id_b);
    }

    #[test]
    fn ascii_input_gets_a_readable_slug_prefix() {
        let id = make_id(&["Taro Suzuki", "Logo Design"], "svc");
        assert!(id.starts_with("taro-suzuki--logo-design-"), "got: {id}");
    }

    #[test]
    fn all_japanese_input_falls_back_to_the_prefix() {
        let id = make_id(&["鈴木一郎", "ロゴ制作"], "svc");
        assert!(id.starts_with("svc-"), "got: {id}");
    }
}
