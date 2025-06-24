#[cfg(test)]
mod tests {
    use super::super::tag::*;
    use tempfile::tempdir;
    use git2::{Repository, Signature};

    #[test]
    fn test_calculate_new_tag_prefers_highest_version() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let signature = Signature::now("Test User", "test@example.com").unwrap();
        // Initial commit
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        ).unwrap();
        // Tag v8.3.2 (regular)
        repo.tag(
            "v8.3.2",
            &repo.head().unwrap().peel_to_commit().unwrap().as_object(),
            &signature,
            "Regular release",
            false,
        ).unwrap();
        // Tag v10.0.0-beta.1 (pre-release)
        repo.tag(
            "v10.0.0-beta.1",
            &repo.head().unwrap().peel_to_commit().unwrap().as_object(),
            &signature,
            "Pre-release",
            false,
        ).unwrap();

        let opts = TagGeneratorOptions {
            default_bump: "minor".to_string(),
            not_with_v: false,
            release_branches: "main,master".to_string(),
            source: ".".to_string(),
            dry_run: true,
            initial_version: "0.0.0".to_string(),
            prerelease: true,
            prerelease_suffix: "beta".to_string(),
            none_string_token: "#none".to_string(),
            force_without_change: false,
            tag_message: None,
            not_publish: true,
        };
        let gen = TagGenerator::new(opts, false);
        let (tag, pre_tag) = gen.get_latest_tags(&repo).unwrap();
        let new_tag = gen.calculate_new_tag(&repo, &tag, &pre_tag, true).unwrap();
        // Should continue from v10.0.0-beta.1, producing v10.0.0-beta.2
        assert!(new_tag.contains("v10.0.0-beta.2"), "new_tag was: {}", new_tag);
    }
}
