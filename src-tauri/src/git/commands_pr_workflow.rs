use super::*;

pub(super) async fn create_git_pr_workflow_impl(
    workspace_id: String,
    upstream_repo: String,
    base_branch: String,
    head_owner: String,
    head_branch: String,
    title: String,
    body: Option<String>,
    comment_after_create: Option<bool>,
    comment_body: Option<String>,
    state: State<'_, AppState>,
) -> Result<GitPrWorkflowResult, String> {
    let workspaces = state.workspaces.lock().await;
    let entry = workspaces
        .get(&workspace_id)
        .ok_or("workspace not found")?
        .clone();
    drop(workspaces);

    let repo_root = resolve_git_root(&entry)?;
    let upstream_repo = upstream_repo.trim().to_string();
    let base_branch = base_branch.trim().to_string();
    let head_owner = head_owner.trim().to_string();
    let head_branch = head_branch.trim().to_string();
    let title = title.trim().to_string();
    let body = body.unwrap_or_default();
    let comment_enabled = comment_after_create.unwrap_or(false);
    let comment_text = comment_body.unwrap_or_default();
    let mut stages = build_workflow_stages();

    if upstream_repo.is_empty()
        || base_branch.is_empty()
        || head_owner.is_empty()
        || head_branch.is_empty()
        || title.is_empty()
    {
        let reason = "Missing required PR parameters (upstream/base/head/title).".to_string();
        update_workflow_stage(
            &mut stages,
            "precheck",
            "failed",
            reason.clone(),
            None,
            None,
            None,
        );
        return Ok(build_failed_pr_workflow_result(
            stages, "precheck", reason, None,
        ));
    }

    update_workflow_stage(
        &mut stages,
        "precheck",
        "running",
        "Checking GitHub CLI readiness and PR range gate.".to_string(),
        None,
        None,
        None,
    );
    let gh_version_args = vec!["--version".to_string()];
    let gh_version_output =
        match run_token_isolated_command(&repo_root, "gh", &gh_version_args, &[]).await {
            Ok(output) => output,
            Err(error) => {
                update_workflow_stage(
                    &mut stages,
                    "precheck",
                    "failed",
                    error.clone(),
                    None,
                    None,
                    None,
                );
                return Ok(build_failed_pr_workflow_result(
                    stages, "precheck", error, None,
                ));
            }
        };
    if !gh_version_output.success {
        let raw = summarize_command_failure(&gh_version_output);
        update_workflow_stage(
            &mut stages,
            "precheck",
            "failed",
            raw.clone(),
            Some(gh_version_output.command),
            Some(truncate_debug_text(&gh_version_output.stdout, 1200)),
            Some(truncate_debug_text(&gh_version_output.stderr, 1200)),
        );
        return Ok(build_failed_pr_workflow_result(
            stages, "precheck", raw, None,
        ));
    }

    let gh_auth_args = vec![
        "auth".to_string(),
        "status".to_string(),
        "-h".to_string(),
        "github.com".to_string(),
    ];
    let gh_auth_output =
        match run_token_isolated_command(&repo_root, "gh", &gh_auth_args, &[]).await {
            Ok(output) => output,
            Err(error) => {
                update_workflow_stage(
                    &mut stages,
                    "precheck",
                    "failed",
                    error.clone(),
                    None,
                    None,
                    None,
                );
                return Ok(build_failed_pr_workflow_result(
                    stages, "precheck", error, None,
                ));
            }
        };
    if !gh_auth_output.success {
        let raw = summarize_command_failure(&gh_auth_output);
        update_workflow_stage(
            &mut stages,
            "precheck",
            "failed",
            raw.clone(),
            Some(gh_auth_output.command),
            Some(truncate_debug_text(&gh_auth_output.stdout, 1200)),
            Some(truncate_debug_text(&gh_auth_output.stderr, 1200)),
        );
        return Ok(build_failed_pr_workflow_result(
            stages, "precheck", raw, None,
        ));
    }

    let repo = open_repository_at_root(&repo_root)?;
    if repo.find_remote("upstream").is_err() {
        let raw = "Range gate requires remote `upstream`. Add it first, then retry PR workflow."
            .to_string();
        update_workflow_stage(
            &mut stages,
            "precheck",
            "failed",
            raw.clone(),
            None,
            None,
            None,
        );
        return Ok(build_failed_pr_workflow_result(
            stages, "precheck", raw, None,
        ));
    }

    let fetch_args = vec![
        "fetch".to_string(),
        "upstream".to_string(),
        base_branch.clone(),
    ];
    let fetch_output = match run_token_isolated_command(&repo_root, "git", &fetch_args, &[]).await {
        Ok(output) => output,
        Err(error) => {
            update_workflow_stage(
                &mut stages,
                "precheck",
                "failed",
                error.clone(),
                None,
                None,
                None,
            );
            return Ok(build_failed_pr_workflow_result(
                stages, "precheck", error, None,
            ));
        }
    };
    if !fetch_output.success {
        let raw = summarize_command_failure(&fetch_output);
        update_workflow_stage(
            &mut stages,
            "precheck",
            "failed",
            raw.clone(),
            Some(fetch_output.command),
            Some(truncate_debug_text(&fetch_output.stdout, 1500)),
            Some(truncate_debug_text(&fetch_output.stderr, 1500)),
        );
        return Ok(build_failed_pr_workflow_result(
            stages, "precheck", raw, None,
        ));
    }

    let range_ref = format!("upstream/{base_branch}...HEAD");
    let range_args = vec![
        "diff".to_string(),
        "--name-only".to_string(),
        range_ref.clone(),
    ];
    let range_output = match run_token_isolated_command(&repo_root, "git", &range_args, &[]).await {
        Ok(output) => output,
        Err(error) => {
            update_workflow_stage(
                &mut stages,
                "precheck",
                "failed",
                error.clone(),
                None,
                None,
                None,
            );
            return Ok(build_failed_pr_workflow_result(
                stages, "precheck", error, None,
            ));
        }
    };
    if !range_output.success {
        let raw = summarize_command_failure(&range_output);
        update_workflow_stage(
            &mut stages,
            "precheck",
            "failed",
            raw.clone(),
            Some(range_output.command),
            Some(truncate_debug_text(&range_output.stdout, 1500)),
            Some(truncate_debug_text(&range_output.stderr, 1500)),
        );
        return Ok(build_failed_pr_workflow_result(
            stages, "precheck", raw, None,
        ));
    }
    let changed_paths = range_output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    match evaluate_pr_range_gate(&changed_paths) {
        PrRangeGateDecision::Pass { changed_files } => {
            update_workflow_stage(
                &mut stages,
                "precheck",
                "success",
                format!("Precheck passed. Range gate changed files: {changed_files}."),
                Some(range_output.command),
                Some(truncate_debug_text(&range_output.stdout, 1600)),
                Some(truncate_debug_text(&range_output.stderr, 1200)),
            );
        }
        PrRangeGateDecision::Blocked { category, reason } => {
            update_workflow_stage(
                &mut stages,
                "precheck",
                "failed",
                reason.clone(),
                Some(range_output.command),
                Some(truncate_debug_text(&range_output.stdout, 1600)),
                Some(truncate_debug_text(&range_output.stderr, 1200)),
            );
            return Ok(GitPrWorkflowResult {
                ok: false,
                status: "failed".to_string(),
                message: reason,
                error_category: Some(category),
                next_action_hint: Some(
                    "Fix branch base/range first, then retry workflow to avoid oversized PR."
                        .to_string(),
                ),
                pr_url: None,
                pr_number: None,
                existing_pr: None,
                retry_command: None,
                stages,
            });
        }
    }

    update_workflow_stage(
        &mut stages,
        "push",
        "running",
        "Pushing branch to fork remote.".to_string(),
        None,
        None,
        None,
    );
    let push_target = format!("HEAD:{head_branch}");
    let push_args = vec![
        "push".to_string(),
        "-u".to_string(),
        "origin".to_string(),
        push_target.clone(),
    ];
    let push_output = match run_token_isolated_command(&repo_root, "git", &push_args, &[]).await {
        Ok(output) => output,
        Err(error) => {
            update_workflow_stage(
                &mut stages,
                "push",
                "failed",
                error.clone(),
                None,
                None,
                None,
            );
            return Ok(build_failed_pr_workflow_result(
                stages,
                "push",
                error,
                Some(format!(
                    "env -u GH_TOKEN -u GITHUB_TOKEN git -c http.version=HTTP/1.1 push -u origin HEAD:{head_branch}"
                )),
            ));
        }
    };

    if !push_output.success {
        let first_error = summarize_command_failure(&push_output);
        if is_http2_transport_error(&first_error) {
            let push_http1_args = vec![
                "-c".to_string(),
                "http.version=HTTP/1.1".to_string(),
                "push".to_string(),
                "-u".to_string(),
                "origin".to_string(),
                push_target.clone(),
            ];
            let retry_output = match run_token_isolated_command(
                &repo_root,
                "git",
                &push_http1_args,
                &[],
            )
            .await
            {
                Ok(output) => output,
                Err(error) => {
                    update_workflow_stage(
                        &mut stages,
                        "push",
                        "failed",
                        error.clone(),
                        None,
                        None,
                        None,
                    );
                    return Ok(build_failed_pr_workflow_result(
                        stages,
                        "push",
                        error,
                        Some(format!(
                            "env -u GH_TOKEN -u GITHUB_TOKEN git -c http.version=HTTP/1.1 push -u origin HEAD:{head_branch}"
                        )),
                    ));
                }
            };
            if !retry_output.success {
                let retry_error = summarize_command_failure(&retry_output);
                update_workflow_stage(
                    &mut stages,
                    "push",
                    "failed",
                    retry_error.clone(),
                    Some(retry_output.command),
                    Some(truncate_debug_text(&retry_output.stdout, 1600)),
                    Some(truncate_debug_text(&retry_output.stderr, 1600)),
                );
                return Ok(build_failed_pr_workflow_result(
                    stages,
                    "push",
                    retry_error,
                    Some(format!(
                        "env -u GH_TOKEN -u GITHUB_TOKEN git -c http.version=HTTP/1.1 push -u origin HEAD:{head_branch}"
                    )),
                ));
            }
            update_workflow_stage(
                &mut stages,
                "push",
                "success",
                "Push succeeded after HTTP/1.1 fallback retry.".to_string(),
                Some(retry_output.command),
                Some(truncate_debug_text(&retry_output.stdout, 1600)),
                Some(truncate_debug_text(&retry_output.stderr, 1200)),
            );
        } else {
            update_workflow_stage(
                &mut stages,
                "push",
                "failed",
                first_error.clone(),
                Some(push_output.command),
                Some(truncate_debug_text(&push_output.stdout, 1600)),
                Some(truncate_debug_text(&push_output.stderr, 1600)),
            );
            return Ok(build_failed_pr_workflow_result(
                stages,
                "push",
                first_error,
                Some(format!(
                    "env -u GH_TOKEN -u GITHUB_TOKEN git -c http.version=HTTP/1.1 push -u origin HEAD:{head_branch}"
                )),
            ));
        }
    } else {
        update_workflow_stage(
            &mut stages,
            "push",
            "success",
            "Branch push completed.".to_string(),
            Some(push_output.command),
            Some(truncate_debug_text(&push_output.stdout, 1600)),
            Some(truncate_debug_text(&push_output.stderr, 1200)),
        );
    }

    update_workflow_stage(
        &mut stages,
        "create",
        "running",
        "Detecting existing PR and creating a new PR when needed.".to_string(),
        None,
        None,
        None,
    );
    let head_spec = format!("{head_owner}:{head_branch}");
    let existing_pr_args = vec![
        "pr".to_string(),
        "list".to_string(),
        "--repo".to_string(),
        upstream_repo.clone(),
        "--state".to_string(),
        "all".to_string(),
        "--head".to_string(),
        head_spec.clone(),
        "--json".to_string(),
        "number,title,url,state,headRefName,baseRefName".to_string(),
        "--limit".to_string(),
        "5".to_string(),
    ];
    let existing_pr_output =
        match run_token_isolated_command(&repo_root, "gh", &existing_pr_args, &[]).await {
            Ok(output) => output,
            Err(error) => {
                update_workflow_stage(
                    &mut stages,
                    "create",
                    "failed",
                    error.clone(),
                    None,
                    None,
                    None,
                );
                return Ok(build_failed_pr_workflow_result(
                    stages, "create", error, None,
                ));
            }
        };
    if !existing_pr_output.success {
        let raw = summarize_command_failure(&existing_pr_output);
        update_workflow_stage(
            &mut stages,
            "create",
            "failed",
            raw.clone(),
            Some(existing_pr_output.command),
            Some(truncate_debug_text(&existing_pr_output.stdout, 1600)),
            Some(truncate_debug_text(&existing_pr_output.stderr, 1600)),
        );
        return Ok(build_failed_pr_workflow_result(stages, "create", raw, None));
    }
    let existing_items: Vec<GhExistingPrEntry> =
        match serde_json::from_str(&existing_pr_output.stdout) {
            Ok(items) => items,
            Err(error) => {
                let raw = format!("Failed to parse existing PR metadata: {error}");
                update_workflow_stage(
                    &mut stages,
                    "create",
                    "failed",
                    raw.clone(),
                    Some(existing_pr_output.command),
                    Some(truncate_debug_text(&existing_pr_output.stdout, 1600)),
                    Some(truncate_debug_text(&existing_pr_output.stderr, 1200)),
                );
                return Ok(build_failed_pr_workflow_result(stages, "create", raw, None));
            }
        };
    if let Some(existing) = existing_items.first() {
        let existing_pr = GitPrExistingPullRequest {
            number: existing.number,
            title: existing.title.clone(),
            url: existing.url.clone(),
            state: existing.state.clone(),
            head_ref_name: existing.head_ref_name.clone(),
            base_ref_name: existing.base_ref_name.clone(),
        };
        update_workflow_stage(
            &mut stages,
            "create",
            "success",
            format!("Existing PR found: #{} {}", existing.number, existing.title),
            Some(existing_pr_output.command),
            Some(truncate_debug_text(&existing_pr_output.stdout, 1200)),
            Some(truncate_debug_text(&existing_pr_output.stderr, 600)),
        );
        update_workflow_stage(
            &mut stages,
            "comment",
            "skipped",
            "Skipped because workflow reused existing PR.".to_string(),
            None,
            None,
            None,
        );
        return Ok(build_existing_pr_workflow_result(stages, existing_pr));
    }

    let body_value = if body.trim().is_empty() {
        default_pr_description(&base_branch, &head_branch)
    } else {
        body
    };
    let create_pr_args = vec![
        "pr".to_string(),
        "create".to_string(),
        "--repo".to_string(),
        upstream_repo.clone(),
        "--base".to_string(),
        base_branch.clone(),
        "--head".to_string(),
        head_spec.clone(),
        "--title".to_string(),
        title.clone(),
        "--body".to_string(),
        body_value,
    ];
    let create_output =
        match run_token_isolated_command(&repo_root, "gh", &create_pr_args, &[]).await {
            Ok(output) => output,
            Err(error) => {
                update_workflow_stage(
                    &mut stages,
                    "create",
                    "failed",
                    error.clone(),
                    None,
                    None,
                    None,
                );
                return Ok(build_failed_pr_workflow_result(
                    stages, "create", error, None,
                ));
            }
        };
    if !create_output.success {
        let raw = summarize_command_failure(&create_output);
        update_workflow_stage(
            &mut stages,
            "create",
            "failed",
            raw.clone(),
            Some(create_output.command),
            Some(truncate_debug_text(&create_output.stdout, 1600)),
            Some(truncate_debug_text(&create_output.stderr, 1600)),
        );
        return Ok(build_failed_pr_workflow_result(stages, "create", raw, None));
    }

    let merged_create_output = if create_output.stdout.trim().is_empty() {
        create_output.stderr.as_str()
    } else {
        create_output.stdout.as_str()
    };
    let mut pr_url = extract_pr_url(merged_create_output).unwrap_or_default();
    if pr_url.is_empty() {
        pr_url = extract_pr_url(&create_output.stdout)
            .or_else(|| extract_pr_url(&create_output.stderr))
            .unwrap_or_default();
    }
    let mut pr_number = extract_pr_number_from_url(&pr_url);
    if pr_number.is_none() {
        let pr_view_args = vec![
            "pr".to_string(),
            "list".to_string(),
            "--repo".to_string(),
            upstream_repo.clone(),
            "--state".to_string(),
            "open".to_string(),
            "--head".to_string(),
            head_spec.clone(),
            "--json".to_string(),
            "number,url".to_string(),
            "--limit".to_string(),
            "1".to_string(),
        ];
        if let Ok(view_output) =
            run_token_isolated_command(&repo_root, "gh", &pr_view_args, &[]).await
        {
            if view_output.success {
                let parsed: serde_json::Value =
                    serde_json::from_str(&view_output.stdout).unwrap_or_else(|_| json!([]));
                if let Some(item) = parsed.as_array().and_then(|items| items.first()) {
                    if pr_url.is_empty() {
                        pr_url = item
                            .get("url")
                            .and_then(|value| value.as_str())
                            .unwrap_or("")
                            .to_string();
                    }
                    pr_number = item.get("number").and_then(|value| value.as_u64());
                }
            }
        }
    }
    update_workflow_stage(
        &mut stages,
        "create",
        "success",
        if pr_url.is_empty() {
            "PR created successfully.".to_string()
        } else {
            format!("PR created: {pr_url}")
        },
        Some(create_output.command),
        Some(truncate_debug_text(&create_output.stdout, 1600)),
        Some(truncate_debug_text(&create_output.stderr, 1200)),
    );

    if !comment_enabled {
        update_workflow_stage(
            &mut stages,
            "comment",
            "skipped",
            "Comment step disabled.".to_string(),
            None,
            None,
            None,
        );
        return Ok(build_success_pr_workflow_result(
            stages,
            pr_url,
            pr_number,
            "PR workflow completed.".to_string(),
        ));
    }

    let effective_pr_number = pr_number;
    let comment_text = comment_text.trim().to_string();
    if comment_text.is_empty() {
        update_workflow_stage(
            &mut stages,
            "comment",
            "skipped",
            "Comment step skipped because body is empty.".to_string(),
            None,
            None,
            None,
        );
        return Ok(build_success_pr_workflow_result(
            stages,
            pr_url,
            effective_pr_number,
            "PR created (comment skipped).".to_string(),
        ));
    }
    let Some(comment_pr_number) = effective_pr_number else {
        update_workflow_stage(
            &mut stages,
            "comment",
            "skipped",
            "Comment step skipped because PR number is unavailable.".to_string(),
            None,
            None,
            None,
        );
        return Ok(build_success_pr_workflow_result(
            stages,
            pr_url,
            None,
            "PR created (comment skipped).".to_string(),
        ));
    };

    update_workflow_stage(
        &mut stages,
        "comment",
        "running",
        "Posting optional approval comment.".to_string(),
        None,
        None,
        None,
    );
    let comment_args = vec![
        "pr".to_string(),
        "comment".to_string(),
        comment_pr_number.to_string(),
        "--repo".to_string(),
        upstream_repo.clone(),
        "--body".to_string(),
        comment_text,
    ];
    let comment_output =
        match run_token_isolated_command(&repo_root, "gh", &comment_args, &[]).await {
            Ok(output) => output,
            Err(error) => {
                update_workflow_stage(&mut stages, "comment", "failed", error, None, None, None);
                return Ok(build_success_pr_workflow_result(
                    stages,
                    pr_url,
                    Some(comment_pr_number),
                    "PR created, but comment step failed.".to_string(),
                ));
            }
        };
    if !comment_output.success {
        let raw = summarize_command_failure(&comment_output);
        update_workflow_stage(
            &mut stages,
            "comment",
            "failed",
            raw,
            Some(comment_output.command),
            Some(truncate_debug_text(&comment_output.stdout, 1200)),
            Some(truncate_debug_text(&comment_output.stderr, 1200)),
        );
        return Ok(build_success_pr_workflow_result(
            stages,
            pr_url,
            Some(comment_pr_number),
            "PR created, but comment step failed.".to_string(),
        ));
    }
    update_workflow_stage(
        &mut stages,
        "comment",
        "success",
        format!("Comment posted on PR #{comment_pr_number}."),
        Some(comment_output.command),
        Some(truncate_debug_text(&comment_output.stdout, 1200)),
        Some(truncate_debug_text(&comment_output.stderr, 600)),
    );
    Ok(build_success_pr_workflow_result(
        stages,
        pr_url,
        Some(comment_pr_number),
        "PR workflow completed.".to_string(),
    ))
}
