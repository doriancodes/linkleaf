use anyhow::{Context, Result, bail};
use linkleaf::api::{add, html, list};
use linkleaf::feed::{read_feed, write_feed};
use linkleaf::linkleaf_proto::{Feed, Link};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io::Write};

pub fn cmd_init(file: PathBuf, title: String, version: u32) -> Result<()> {
    if file.exists() {
        bail!("file already exists: {}", file.display());
    }

    let mut feed = Feed::default();
    feed.title = title;
    feed.version = version;

    let modified_feed = write_feed(&file, feed)?;
    eprintln!(
        "Initialized feed: '{}' (v{}) → {}",
        modified_feed.title,
        modified_feed.version,
        file.display()
    );
    Ok(())
}

pub fn cmd_add(
    file: PathBuf,
    title: String,
    url: String,
    summary: Option<String>,
    tags: Option<String>,
    via: Option<String>,
    id: Option<String>,
) -> Result<()> {
    add(file, title, url, summary, tags, via, id)?;
    Ok(())
}

pub fn cmd_list(file: PathBuf, long: bool) -> Result<()> {
    let feed = list(&file)?;

    if long {
        long_print(feed);
    } else {
        println!(
            "Feed: '{}' (v{}) — {} links\n",
            feed.title,
            feed.version,
            feed.links.len()
        );

        for (idx, l) in feed.links.iter().enumerate() {
            let tags = if l.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", l.tags.join(","))
            };
            println!(
                "{:>3}. {}  {}{}\n     {}",
                idx + 1,
                l.date,
                l.title,
                tags,
                l.url
            );
        }
    }
    Ok(())
}

fn long_print(feed: Feed) {
    println!("Feed: '{}' (v{})\n", feed.title, feed.version);
    for l in &feed.links {
        println!("- [{}] {}", l.date, l.title);
        println!("  id: {}", l.id);
        println!("  url: {}", l.url);
        if !l.via.is_empty() {
            println!("  via: {}", l.via);
        }
        if !l.summary.is_empty() {
            println!("  summary: {}", l.summary);
        }
        if !l.tags.is_empty() {
            println!("  tags: {}", l.tags.join(", "));
        }
        println!();
    }
}

pub fn cmd_html(file: PathBuf, out: PathBuf, custom_title: Option<String>) -> Result<()> {
    let html = html(file, custom_title)?;

    // write atomically (same pattern as write_feed)
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).ok();
    }
    let tmp = out.with_extension("html.tmp");
    {
        let mut f =
            fs::File::create(&tmp).with_context(|| format!("failed to write {}", tmp.display()))?;
        f.write_all(html.as_bytes())?;
        f.flush()?;
    }
    fs::rename(&tmp, &out)
        .with_context(|| format!("failed to move temp file into place: {}", out.display()))?;

    // copy style.css next to the HTML output
    let css_src = PathBuf::from("templates/style.css");
    if css_src.exists() {
        let css_out = out
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("style.css");
        fs::copy(&css_src, &css_out).with_context(|| {
            format!(
                "failed to copy {} to {}",
                css_src.display(),
                css_out.display()
            )
        })?;
    }

    eprintln!("Wrote HTML: {}", out.display());
    Ok(())
}

pub fn cmd_publish(
    file: PathBuf,
    remote: &str,
    branch: Option<String>,
    message: &str,
    allow_empty: bool,
    no_push: bool,
) -> Result<()> {
    // Ensure file exists
    if !file.exists() {
        bail!("feed file not found: {}", file.display());
    }

    // Resolve absolute paths
    let file_abs =
        fs::canonicalize(&file).with_context(|| format!("failed to resolve {}", file.display()))?;
    let file_dir = file_abs.parent().unwrap_or_else(|| Path::new("."));

    // Find repo root via git
    let repo_root = git_output(
        &[
            "-C",
            file_dir.to_str().unwrap(),
            "rev-parse",
            "--show-toplevel",
        ],
        "detect git repo",
    )?;
    let repo_root = PathBuf::from(repo_root.trim());

    // Build path relative to repo root for the add/commit
    let rel = file_abs
        .strip_prefix(&repo_root)
        .unwrap_or(&file_abs)
        .to_path_buf();
    let rel_str = rel.to_string_lossy();

    // Stage file
    git_check(
        &["-C", repo_root.to_str().unwrap(), "add", "--", &rel_str],
        "git add",
    )?;

    // Commit (handle "nothing to commit" gracefully if allow_empty)
    let mut commit_args = vec!["-C", repo_root.to_str().unwrap(), "commit"];
    if allow_empty {
        commit_args.push("--allow-empty");
    }
    commit_args.extend_from_slice(&["-m", &message, "--", &rel_str]);

    match Command::new("git").args(&commit_args).output() {
        Ok(o) if o.status.success() => {
            eprintln!("Committed: {}", String::from_utf8_lossy(&o.stdout).trim());
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            if stderr.contains("nothing to commit") {
                eprintln!("Nothing to commit; working tree clean.");
            } else {
                bail!("git commit failed: {}", stderr.trim());
            }
        }
        Err(e) => bail!("failed to run git commit: {e}"),
    }

    if no_push {
        eprintln!("Skipped push (--no-push).");
        return Ok(());
    }

    // Push
    if let Some(branch) = branch.as_deref() {
        // Push HEAD to the given branch and set upstream
        git_check(
            &[
                "-C",
                repo_root.to_str().unwrap(),
                "push",
                "-u",
                &remote,
                &format!("HEAD:{branch}"),
            ],
            "git push -u",
        )?;
    } else {
        // Let git use the current upstream
        git_check(
            &["-C", repo_root.to_str().unwrap(), "push", &remote],
            "git push",
        )?;
    }

    eprintln!(
        "Published {} to {}{}",
        rel_str,
        remote,
        branch
            .as_deref()
            .map(|b| format!("/{}", b))
            .unwrap_or_default()
    );
    Ok(())
}

fn git_output(args: &[&str], what: &str) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("failed to run git for {what}"))?;
    if !out.status.success() {
        bail!(
            "git command failed ({what}): {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn git_check(args: &[&str], what: &str) -> Result<()> {
    let out = Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("failed to run git for {what}"))?;
    if !out.status.success() {
        bail!(
            "git command failed ({what}): {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use linkleaf::linkleaf_proto::{Feed, Link};
    use tempfile::TempDir;

    fn sample_feed_one() -> Feed {
        let mut f = Feed::default();
        f.title = "Sample".into();
        f.version = 1;
        f.links.push(Link {
            id: "one".into(),
            title: "First".into(),
            url: "https://example.com/1".into(),
            date: "2025-01-01".into(),
            summary: "hello".into(),
            tags: vec!["x".into(), "y".into()],
            via: "".into(),
        });
        f
    }

    #[test]
    fn init_creates_file_and_defaults() -> anyhow::Result<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("nested/dir/mylinks.pb");
        cmd_init(path.clone(), "My Links".into(), 2)?;
        assert!(path.exists(), "feed file should exist");
        let feed = read_feed(&PathBuf::from(&path))?;
        assert_eq!(feed.title, "My Links");
        assert_eq!(feed.version, 2);
        assert!(feed.links.is_empty());
        Ok(())
    }

    #[test]
    fn add_inserts_then_updates_same_id() -> anyhow::Result<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("feed.pb");

        // Insert (file doesn't exist yet; implementation should create a v1 feed)
        cmd_add(
            path.clone(),
            "Rust Book".into(),
            "https://doc.rust-lang.org/book/".into(),
            Some("Great read".into()),
            Some("rust,book".into()),
            Some("https://rust-lang.org".into()),
            Some("fixed-id".into()), // ensure deterministic update target
        )?;
        let mut feed = read_feed(&PathBuf::from(&path))?;
        assert_eq!(feed.links.len(), 1);
        assert_eq!(feed.links[0].id, "fixed-id");
        assert_eq!(feed.links[0].title, "Rust Book");

        // Update same id: title & summary change; still one entry
        cmd_add(
            path.clone(),
            "The Rust Book".into(),
            "https://doc.rust-lang.org/book/".into(),
            Some("Updated summary".into()),
            Some("rust,book".into()),
            None,
            Some("fixed-id".into()),
        )?;
        feed = read_feed(&PathBuf::from(&path))?;
        assert_eq!(feed.links.len(), 1, "should update, not duplicate");
        assert_eq!(feed.links[0].title, "The Rust Book");
        assert_eq!(feed.links[0].summary, "Updated summary");
        Ok(())
    }

    #[test]
    fn list_compact_and_long_run() -> anyhow::Result<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("feed.pb");
        write_feed(&PathBuf::from(&path), sample_feed_one())?;

        // We don’t assert output formatting here; just ensure no panic/err.
        cmd_list(path.clone(), false)?;
        cmd_list(path.clone(), true)?;
        Ok(())
    }

    #[test]
    fn html_renders_and_writes_css() -> anyhow::Result<()> {
        let tmp = TempDir::new()?;
        let feed_path = tmp.path().join("feed.pb");
        write_feed(&PathBuf::from(&feed_path), sample_feed_one())?;

        let out_html = tmp.path().join("site/index.html");
        cmd_html(feed_path.clone(), out_html.clone(), Some("My Page".into()))?;

        let html = std::fs::read_to_string(&out_html)?;
        assert!(
            html.contains("<title>My Page"),
            "title should appear in HTML"
        );
        assert!(html.contains("First"), "link title should render");
        let css_out = out_html.parent().unwrap().join("style.css");
        assert!(css_out.exists(), "style.css should be written next to HTML");
        Ok(())
    }

    #[test]
    fn publish_adds_commits_and_pushes_to_local_bare() -> anyhow::Result<()> {
        // Skip if git isn't available
        if Command::new("git").arg("--version").output().is_err() {
            eprintln!("git not found; skipping publish test");
            return Ok(());
        }

        use tempfile::TempDir;

        let tmp = TempDir::new()?;
        let bare = tmp.path().join("remote.git");
        let work = tmp.path().join("work");
        let clone_dir = tmp.path().join("clone");

        // --- 1) Init bare repo (try to set initial branch to main; fallback for older Git)
        {
            let out = Command::new("git")
                .args(["init", "--bare", "--initial-branch=main"])
                .arg(&bare)
                .output()?;

            if !out.status.success() {
                // Fallback: older Git without --initial-branch
                let out2 = Command::new("git")
                    .args(["init", "--bare"])
                    .arg(&bare)
                    .output()?;
                assert!(
                    out2.status.success(),
                    "git init --bare failed: {}",
                    String::from_utf8_lossy(&out2.stderr)
                );

                // Point HEAD at refs/heads/main for cleanliness (not strictly required for clone --branch)
                let out3 = Command::new("git")
                    .current_dir(&bare)
                    .args(["symbolic-ref", "HEAD", "refs/heads/main"])
                    .output()?;
                assert!(
                    out3.status.success(),
                    "set bare HEAD failed: {}",
                    String::from_utf8_lossy(&out3.stderr)
                );
            }
        }

        // --- 2) Init work repo, commit, publish
        {
            fs::create_dir_all(&work)?;
            let run = |args: &[&str]| -> anyhow::Result<()> {
                let out = Command::new("git").args(args).current_dir(&work).output()?;
                if !out.status.success() {
                    bail!(
                        "git failed: {} ({:?})",
                        String::from_utf8_lossy(&out.stderr),
                        args
                    );
                }
                Ok(())
            };

            run(&["init"])?;
            run(&["config", "user.name", "Test User"])?;
            run(&["config", "user.email", "test@example.com"])?;
            run(&["checkout", "-b", "main"])?;
            run(&["remote", "add", "origin", bare.to_str().unwrap()])?;

            let feed_path = work.join("feed/mylinks.pb");
            write_feed(&feed_path, {
                let mut f = Feed::default();
                f.title = "Sample".into();
                f.version = 1;
                f
            })?;

            // Push HEAD to origin/main
            cmd_publish(
                feed_path.clone(),
                "origin",
                Some("main".into()),
                "init commit",
                false,
                false,
            )?;
        }

        // --- 3) Clone the remote and verify
        {
            let out = Command::new("git")
                .args([
                    "clone",
                    "--branch",
                    "main",
                    "--single-branch",
                    bare.to_str().unwrap(),
                    clone_dir.to_str().unwrap(),
                ])
                .output()?;
            assert!(
                out.status.success(),
                "git clone failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );

            let cloned_feed = clone_dir.join("feed/mylinks.pb");
            assert!(cloned_feed.exists(), "feed should be present in clone");

            let feed = read_feed(&PathBuf::from(&cloned_feed))?;
            assert_eq!(feed.title, "Sample");
        }

        Ok(())
    }
}
