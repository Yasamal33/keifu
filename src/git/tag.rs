use anyhow::Result;
use git2::{Oid, Repository};

#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub tip_oid: Oid,
}

impl TagInfo {
    pub fn list_all(repo: &Repository) -> Result<Vec<Self>> {
        let mut tags = Vec::new();

        for reference in repo.references_glob("refs/tags/*")? {
            let reference = reference?;
            let Some(name) = reference.shorthand() else {
                continue;
            };
            // Skip tags that don't resolve to a commit (e.g. tags pointing
            // at blobs or trees) rather than failing the whole listing.
            let Ok(commit) = reference.peel_to_commit() else {
                continue;
            };

            tags.push(TagInfo {
                name: name.to_string(),
                tip_oid: commit.id(),
            });
        }

        tags.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2::Signature;
    use tempfile::TempDir;

    use super::*;

    fn init_repo_with_commit() -> (TempDir, Repository) {
        let tempdir = tempfile::tempdir().unwrap();
        let repo = Repository::init(tempdir.path()).unwrap();
        fs::write(tempdir.path().join("base.txt"), "base\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("base.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
        drop(tree);
        (tempdir, repo)
    }

    #[test]
    fn list_all_returns_empty_when_no_tags() {
        let (_tempdir, repo) = init_repo_with_commit();
        assert!(TagInfo::list_all(&repo).unwrap().is_empty());
    }

    #[test]
    fn list_all_resolves_lightweight_and_annotated_tags() {
        let (_tempdir, repo) = init_repo_with_commit();
        let head_oid = repo.head().unwrap().target().unwrap();
        let target = repo.find_object(head_oid, None).unwrap();

        repo.tag_lightweight("v1.0.0", &target, false).unwrap();

        let sig = Signature::now("Test", "test@example.com").unwrap();
        repo.tag("v1.1.0", &target, &sig, "release notes", false)
            .unwrap();

        let tags = TagInfo::list_all(&repo).unwrap();

        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "v1.0.0");
        assert_eq!(tags[0].tip_oid, head_oid);
        assert_eq!(tags[1].name, "v1.1.0");
        assert_eq!(tags[1].tip_oid, head_oid);
    }
}
