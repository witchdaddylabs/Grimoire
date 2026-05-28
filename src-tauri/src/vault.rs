pub use crate::db::read_vault_tree;
pub use crate::db::{read_banned_words, scan_banned_words};
use crate::db::{collect_named_rows, count_words};
use crate::models::VaultItemNode;
use crate::models::VaultTreeResponse;

// Re-export with vault-specific naming
pub fn vault_tree_response_from_db(
    wings: Vec<crate::models::VaultWingNode>,
    item_count: usize,
) -> VaultTreeResponse {
    VaultTreeResponse { wings, item_count }
}

pub fn flatten_vault_items(tree: &VaultTreeResponse) -> Vec<&VaultItemNode> {
    let mut items = Vec::new();
    for wing in &tree.wings {
        for hall in &wing.halls {
            for room in &hall.rooms {
                for drawer in &room.drawers {
                    for item in &drawer.items {
                        items.push(item);
                    }
                }
            }
        }
    }
    items
}

pub use crate::db::add_banned_word;
pub use crate::db::remove_banned_word;
