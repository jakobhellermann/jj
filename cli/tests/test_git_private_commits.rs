// Copyright 2024 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::Path;
use std::path::PathBuf;

use crate::common::TestEnvironment;

fn set_up() -> (TestEnvironment, PathBuf) {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "origin"]).success();
    let origin_path = test_env.env_root().join("origin");
    let origin_git_repo_path = origin_path
        .join(".jj")
        .join("repo")
        .join("store")
        .join("git");

    test_env
        .run_jj_in(&origin_path, ["describe", "-m=public 1"])
        .success();
    test_env
        .run_jj_in(&origin_path, ["new", "-m=public 2"])
        .success();
    test_env
        .run_jj_in(&origin_path, ["bookmark", "create", "-r@", "main"])
        .success();
    test_env
        .run_jj_in(&origin_path, ["git", "export"])
        .success();

    test_env
        .run_jj_in(
            ".",
            [
                "git",
                "clone",
                "--config=git.auto-local-bookmark=true",
                origin_git_repo_path.to_str().unwrap(),
                "local",
            ],
        )
        .success();
    let workspace_root = test_env.env_root().join("local");

    (test_env, workspace_root)
}

fn set_up_remote_at_main(test_env: &TestEnvironment, workspace_root: &Path, remote_name: &str) {
    test_env
        .run_jj_in(".", ["git", "init", remote_name])
        .success();
    let other_path = test_env.env_root().join(remote_name);
    let other_git_repo_path = other_path
        .join(".jj")
        .join("repo")
        .join("store")
        .join("git");
    test_env
        .run_jj_in(
            workspace_root,
            [
                "git",
                "remote",
                "add",
                remote_name,
                other_git_repo_path.to_str().unwrap(),
            ],
        )
        .success();
    test_env
        .run_jj_in(
            workspace_root,
            [
                "git",
                "push",
                "--allow-new",
                "--remote",
                remote_name,
                "-b=main",
            ],
        )
        .success();
}

#[test]
fn test_git_private_commits_block_pushing() {
    let (test_env, workspace_root) = set_up();

    test_env
        .run_jj_in(&workspace_root, ["new", "main", "-m=private 1"])
        .success();
    test_env
        .run_jj_in(&workspace_root, ["bookmark", "set", "main", "-r@"])
        .success();

    // Will not push when a pushed commit is contained in git.private-commits
    test_env.add_config(r#"git.private-commits = "description(glob:'private*')""#);
    let output = test_env.run_jj_in(&workspace_root, ["git", "push", "--all"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Won't push commit aa3058ff8663 since it is private
    Hint: Rejected commit: yqosqzyt aa3058ff main* | (empty) private 1
    Hint: Configured git.private-commits: 'description(glob:'private*')'
    [EOF]
    [exit status: 1]
    ");

    // May push when the commit is removed from git.private-commits
    test_env.add_config(r#"git.private-commits = "none()""#);
    let output = test_env.run_jj_in(&workspace_root, ["git", "push", "--all"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Changes to push to origin:
      Move forward bookmark main from 7eb97bf230ad to aa3058ff8663
    Warning: The working-copy commit in workspace 'default' became immutable, so a new commit has been created on top of it.
    Working copy now at: znkkpsqq 2e1adf47 (empty) (no description set)
    Parent commit      : yqosqzyt aa3058ff main | (empty) private 1
    [EOF]
    ");
}

#[test]
fn test_git_private_commits_can_be_overridden() {
    let (test_env, workspace_root) = set_up();

    test_env
        .run_jj_in(&workspace_root, ["new", "main", "-m=private 1"])
        .success();
    test_env
        .run_jj_in(&workspace_root, ["bookmark", "set", "main", "-r@"])
        .success();

    // Will not push when a pushed commit is contained in git.private-commits
    test_env.add_config(r#"git.private-commits = "description(glob:'private*')""#);
    let output = test_env.run_jj_in(&workspace_root, ["git", "push", "--all"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Won't push commit aa3058ff8663 since it is private
    Hint: Rejected commit: yqosqzyt aa3058ff main* | (empty) private 1
    Hint: Configured git.private-commits: 'description(glob:'private*')'
    [EOF]
    [exit status: 1]
    ");

    // May push when the commit is removed from git.private-commits
    let output = test_env.run_jj_in(&workspace_root, ["git", "push", "--all", "--allow-private"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Changes to push to origin:
      Move forward bookmark main from 7eb97bf230ad to aa3058ff8663
    Warning: The working-copy commit in workspace 'default' became immutable, so a new commit has been created on top of it.
    Working copy now at: znkkpsqq 2e1adf47 (empty) (no description set)
    Parent commit      : yqosqzyt aa3058ff main | (empty) private 1
    [EOF]
    ");
}

#[test]
fn test_git_private_commits_are_not_checked_if_immutable() {
    let (test_env, workspace_root) = set_up();

    test_env
        .run_jj_in(&workspace_root, ["new", "main", "-m=private 1"])
        .success();
    test_env
        .run_jj_in(&workspace_root, ["bookmark", "set", "main", "-r@"])
        .success();

    test_env.add_config(r#"git.private-commits = "description(glob:'private*')""#);
    test_env.add_config(r#"revset-aliases."immutable_heads()" = "all()""#);
    let output = test_env.run_jj_in(&workspace_root, ["git", "push", "--all"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Changes to push to origin:
      Move forward bookmark main from 7eb97bf230ad to aa3058ff8663
    Warning: The working-copy commit in workspace 'default' became immutable, so a new commit has been created on top of it.
    Working copy now at: yostqsxw dce4a15c (empty) (no description set)
    Parent commit      : yqosqzyt aa3058ff main | (empty) private 1
    [EOF]
    ");
}

#[test]
fn test_git_private_commits_not_directly_in_line_block_pushing() {
    let (test_env, workspace_root) = set_up();

    // New private commit descended from root()
    test_env
        .run_jj_in(&workspace_root, ["new", "root()", "-m=private 1"])
        .success();

    test_env
        .run_jj_in(&workspace_root, ["new", "main", "@", "-m=public 3"])
        .success();
    test_env
        .run_jj_in(&workspace_root, ["bookmark", "create", "-r@", "bookmark1"])
        .success();

    test_env.add_config(r#"git.private-commits = "description(glob:'private*')""#);
    let output = test_env.run_jj_in(
        &workspace_root,
        ["git", "push", "--allow-new", "-b=bookmark1"],
    );
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Won't push commit f1253a9b1ea9 since it is private
    Hint: Rejected commit: yqosqzyt f1253a9b (empty) private 1
    Hint: Configured git.private-commits: 'description(glob:'private*')'
    [EOF]
    [exit status: 1]
    ");
}

#[test]
fn test_git_private_commits_descending_from_commits_pushed_do_not_block_pushing() {
    let (test_env, workspace_root) = set_up();

    test_env
        .run_jj_in(&workspace_root, ["new", "main", "-m=public 3"])
        .success();
    test_env
        .run_jj_in(&workspace_root, ["bookmark", "move", "main", "--to=@"])
        .success();
    test_env
        .run_jj_in(&workspace_root, ["new", "-m=private 1"])
        .success();

    test_env.add_config(r#"git.private-commits = "description(glob:'private*')""#);
    let output = test_env.run_jj_in(&workspace_root, ["git", "push", "-b=main"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Changes to push to origin:
      Move forward bookmark main from 7eb97bf230ad to 05ef53bc99ec
    [EOF]
    ");
}

#[test]
fn test_git_private_commits_already_on_the_remote_do_not_block_push() {
    let (test_env, workspace_root) = set_up();

    // Start a bookmark before a "private" commit lands in main
    test_env
        .run_jj_in(
            &workspace_root,
            ["bookmark", "create", "bookmark1", "-r=main"],
        )
        .success();

    // Push a commit that would become a private_root if it weren't already on
    // the remote
    test_env
        .run_jj_in(&workspace_root, ["new", "main", "-m=private 1"])
        .success();
    test_env
        .run_jj_in(&workspace_root, ["new", "-m=public 3"])
        .success();
    test_env
        .run_jj_in(&workspace_root, ["bookmark", "set", "main", "-r@"])
        .success();
    let output = test_env.run_jj_in(
        &workspace_root,
        ["git", "push", "--allow-new", "-b=main", "-b=bookmark1"],
    );
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Changes to push to origin:
      Move forward bookmark main from 7eb97bf230ad to fbb352762352
      Add bookmark bookmark1 to 7eb97bf230ad
    Warning: The working-copy commit in workspace 'default' became immutable, so a new commit has been created on top of it.
    Working copy now at: kpqxywon a7b08364 (empty) (no description set)
    Parent commit      : yostqsxw fbb35276 main | (empty) public 3
    [EOF]
    ");

    test_env.add_config(r#"git.private-commits = "description(glob:'private*')""#);

    // Since "private 1" is already on the remote, pushing it should be allowed
    test_env
        .run_jj_in(&workspace_root, ["bookmark", "set", "bookmark1", "-r=main"])
        .success();
    let output = test_env.run_jj_in(&workspace_root, ["git", "push", "--all"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Changes to push to origin:
      Move forward bookmark bookmark1 from 7eb97bf230ad to fbb352762352
    [EOF]
    ");

    // Ensure that the already-pushed commit doesn't block a new bookmark from
    // being pushed
    test_env
        .run_jj_in(
            &workspace_root,
            ["new", "description('private 1')", "-m=public 4"],
        )
        .success();
    test_env
        .run_jj_in(&workspace_root, ["bookmark", "create", "-r@", "bookmark2"])
        .success();
    let output = test_env.run_jj_in(
        &workspace_root,
        ["git", "push", "--allow-new", "-b=bookmark2"],
    );
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Changes to push to origin:
      Add bookmark bookmark2 to ee5b808b0b95
    [EOF]
    ");
}

#[test]
fn test_git_private_commits_are_evaluated_separately_for_each_remote() {
    let (test_env, workspace_root) = set_up();
    set_up_remote_at_main(&test_env, &workspace_root, "other");
    test_env.add_config(r#"revset-aliases."immutable_heads()" = "none()""#);

    // Push a commit that would become a private_root if it weren't already on
    // the remote
    test_env
        .run_jj_in(&workspace_root, ["new", "main", "-m=private 1"])
        .success();
    test_env
        .run_jj_in(&workspace_root, ["new", "-m=public 3"])
        .success();
    test_env
        .run_jj_in(&workspace_root, ["bookmark", "set", "main", "-r@"])
        .success();
    let output = test_env.run_jj_in(&workspace_root, ["git", "push", "-b=main"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Changes to push to origin:
      Move forward bookmark main from 7eb97bf230ad to d8632ce893ab
    [EOF]
    ");

    test_env.add_config(r#"git.private-commits = "description(glob:'private*')""#);

    // But pushing to a repo that doesn't have the private commit yet is still
    // blocked
    let output = test_env.run_jj_in(
        &workspace_root,
        ["git", "push", "--remote=other", "-b=main"],
    );
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Won't push commit 36b7ecd11ad9 since it is private
    Hint: Rejected commit: znkkpsqq 36b7ecd1 (empty) private 1
    Hint: Configured git.private-commits: 'description(glob:'private*')'
    [EOF]
    [exit status: 1]
    ");
}
