use serde_json::Value;

pub struct ToolActivityView {
    pub title: String,
    pub detail: Option<String>,
    pub kind: String,
}

pub fn build_activity_view(
    tool_name: &str,
    args: &Value,
    result: Option<&str>,
) -> ToolActivityView {
    let kind = activity_kind(tool_name);
    let mut title = build_title(tool_name, args);
    if result.is_some_and(|r| r.contains("fuzzy")) {
        title.push_str("（fuzzy）");
    }
    let detail = match tool_name {
        "run_shell" | "read_shell_output" | "wait_for_shell" => {
            let cmd = args["command"]
                .as_str()
                .or_else(|| args["job_id"].as_str())
                .unwrap_or("");
            let output = result.unwrap_or("");
            if output.is_empty() && result.is_none() {
                if cmd.is_empty() {
                    None
                } else {
                    Some(format!("```powershell\n{cmd}\n```"))
                }
            } else {
                Some(format!(
                    "```powershell\n{cmd}\n```\n\n**输出：**\n```\n{}\n```",
                    truncate(output, 4000)
                ))
            }
        }
        "read_file" | "list_folder" | "find_files" | "search_files" | "list_symbols"
        | "fetch_url" | "web_search" | "browser_read" | "get_context" | "get_workspace" => result
            .filter(|value| !should_hide_result_detail(tool_name, value))
            .map(str::to_string),
        _ => result
            .filter(|value| !should_hide_result_detail(tool_name, value))
            .map(|r| r.to_string())
            .or_else(|| build_detail_from_args(tool_name, args)),
    };
    ToolActivityView {
        title,
        detail,
        kind,
    }
}

fn should_hide_result_detail(tool_name: &str, result: &str) -> bool {
    if result.starts_with("tool error:") {
        return false;
    }
    if matches!(
        tool_name,
        "read_file"
            | "list_folder"
            | "find_files"
            | "search_files"
            | "list_symbols"
            | "fetch_url"
            | "web_search"
            | "browser_read"
            | "get_context"
            | "get_workspace"
    ) {
        return true;
    }
    matches!(
        tool_name,
        "apply_patch"
            | "write_file"
            | "replace_in_file"
            | "replace_many_in_file"
            | "move_path"
            | "delete_text_range"
            | "delete_go_symbol"
            | "edit_notebook_cell"
    ) && (matches!(
        result.trim(),
        "written" | "replaced" | "moved" | "deleted" | "updated"
    ) || result.trim().starts_with("replaced (")
        || result.trim().starts_with("applied "))
}

fn activity_kind(tool_name: &str) -> String {
    match tool_name {
        "run_shell" | "read_shell_output" | "wait_for_shell" | "stop_shell" => "shell".into(),
        "write_file" => "create".into(),
        "apply_patch" | "replace_in_file" | "replace_many_in_file" | "edit_notebook_cell" => {
            "edit".into()
        }
        "delete_text_range" | "delete_go_symbol" => "delete".into(),
        "move_path" => "move".into(),
        "read_file" | "list_folder" | "find_files" | "search_files" | "list_symbols"
        | "fetch_url" | "web_search" | "browser_read" | "get_context" | "get_workspace" => {
            "read".into()
        }
        _ => "other".into(),
    }
}

fn build_title(tool_name: &str, args: &Value) -> String {
    match tool_name {
        "run_shell" => format!(
            "执行命令：{}",
            truncate(args["command"].as_str().unwrap_or(""), 120)
        ),
        "wait_for_shell" => format!(
            "等待命令：{}",
            args["job_id"].as_str().unwrap_or("job")
        ),
        "read_shell_output" => format!(
            "读取输出：{}",
            args["job_id"].as_str().unwrap_or("job")
        ),
        "stop_shell" => format!(
            "停止命令：{}",
            args["job_id"].as_str().unwrap_or("job")
        ),
        "write_file" => format!("创建/写入文件：{}", path_arg(args)),
        "apply_patch" => {
            let input = args["input"].as_str().or_else(|| args["patch"].as_str()).unwrap_or("");
            let files = input
                .lines()
                .filter_map(|line| {
                    let t = line.trim();
                    t.strip_prefix("*** Add File: ")
                        .or_else(|| t.strip_prefix("*** Update File: "))
                        .or_else(|| t.strip_prefix("*** Delete File: "))
                        .map(str::trim)
                })
                .collect::<Vec<_>>();
            if files.is_empty() {
                "应用补丁".into()
            } else if files.len() == 1 {
                format!("应用补丁：{}", files[0])
            } else {
                format!("应用补丁：{} 个文件", files.len())
            }
        }
        "replace_in_file" => format!("修改文件：{}", path_arg(args)),
        "replace_many_in_file" => format!(
            "批量修改文件：{}（{} 处）",
            path_arg(args),
            args["edits"].as_array().map(|a| a.len()).unwrap_or(0)
        ),
        "move_path" => format!(
            "移动：{} → {}",
            args["from"].as_str().unwrap_or(""),
            args["to"].as_str().unwrap_or("")
        ),
        "delete_text_range" | "delete_go_symbol" => format!("删除内容：{}", path_arg(args)),
        "edit_notebook_cell" => format!("编辑 Notebook：{}", path_arg(args)),
        "read_file" => format!("读取文件：{}", path_arg(args)),
        "list_folder" => format!("列出目录：{}", path_arg(args)),
        "find_files" => format!("查找文件：{}", args["pattern"].as_str().unwrap_or("")),
        "search_files" => format!("搜索：{}", args["pattern"].as_str().unwrap_or("")),
        "run_subagent" | "run_readonly_subagent" => "运行子 Agent".into(),
        "ask_user" => "询问用户".into(),
        "update_tasks" => "更新任务列表".into(),
        "web_search" => format!("Web search: {}", args["query"].as_str().unwrap_or("")),
        "browser_read" => format!("Read web page: {}", args["url"].as_str().unwrap_or("")),
        "get_context" => "Read current context".into(),
        "get_workspace" => "Read workspace".into(),
        "git" => format!("Git: {}", args["action"].as_str().unwrap_or("")),
        other => other.replace('_', " "),
    }
}

fn build_detail_from_args(tool_name: &str, args: &Value) -> Option<String> {
    match tool_name {
        "run_shell" => None,
        "write_file" => {
            let content = args["content"].as_str().unwrap_or("");
            Some(format!("```\n{}\n```", truncate(content, 2000)))
        }
        "apply_patch" => {
            let input = args["input"].as_str().or_else(|| args["patch"].as_str()).unwrap_or("");
            Some(format!("```\n{}\n```", truncate(input, 4000)))
        }
        "replace_in_file" => {
            let old = args["old_string"].as_str().unwrap_or("");
            let new = args["new_string"].as_str().unwrap_or("");
            Some(format_diff(old, new))
        }
        "replace_many_in_file" => {
            let edits = args["edits"].as_array()?;
            let mut parts = Vec::new();
            for (idx, edit) in edits.iter().enumerate() {
                let old = edit["old_string"].as_str().unwrap_or("");
                let new = edit["new_string"].as_str().unwrap_or("");
                parts.push(format!("### 编辑 {}\n{}", idx + 1, format_diff(old, new)));
            }
            Some(parts.join("\n\n"))
        }
        "delete_text_range" => {
            let start = args["start_anchor"].as_str().unwrap_or("");
            let end = args["end_anchor"].as_str().unwrap_or("");
            Some(format!("删除锚点区间：\n```\n{start}\n…\n{end}\n```"))
        }
        "delete_go_symbol" => Some(format!(
            "删除符号：`{}`",
            args["symbol"].as_str().unwrap_or("")
        )),
        "run_subagent" | "run_readonly_subagent" => args["prompt"]
            .as_str()
            .map(|prompt| truncate(prompt, 1_200)),
        "run_parallel_subagents" => args["tasks"].as_array().map(|tasks| {
            let descriptions = tasks
                .iter()
                .enumerate()
                .filter_map(|(index, task)| {
                    task["prompt"]
                        .as_str()
                        .map(|prompt| format!("{}. {}", index + 1, truncate(prompt, 400)))
                })
                .collect::<Vec<_>>();
            descriptions.join("\n\n")
        }),
        _ => None,
    }
}

fn format_diff(old: &str, new: &str) -> String {
    let mut out = String::from("```diff\n");
    for line in old.lines() {
        out.push_str(&format!("-{line}\n"));
    }
    for line in new.lines() {
        out.push_str(&format!("+{line}\n"));
    }
    out.push_str("```");
    out
}

fn path_arg(args: &Value) -> &str {
    args["path"].as_str().unwrap_or(".")
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn replace_in_file_shows_diff() {
        let args = json!({
            "path": "src/main.rs",
            "old_string": "fn old() {}",
            "new_string": "fn new() {}"
        });
        let view = build_activity_view("replace_in_file", &args, Some("replaced"));
        assert!(view.title.contains("src/main.rs"));
        let detail = view.detail.expect("detail");
        assert!(detail.contains("-fn old()"));
        assert!(detail.contains("+fn new()"));
    }

    #[test]
    fn run_shell_shows_command_and_output() {
        let args = json!({ "command": "echo hello" });
        let view = build_activity_view("run_shell", &args, Some("hello\n"));
        let detail = view.detail.unwrap();
        assert!(detail.contains("echo hello"));
        assert!(detail.contains("hello"));
    }

    #[test]
    fn read_file_hides_content() {
        let args = json!({ "path": "src/main.rs" });
        let content = (1..=120)
            .map(|n| format!("{n:>6}|line {n}\n"))
            .collect::<String>();
        let view = build_activity_view("read_file", &args, Some(&content));
        assert!(view.title.contains("src/main.rs"));
        assert!(view.detail.is_none());
    }

    #[test]
    fn read_file_shows_error() {
        let args = json!({ "path": "missing.rs" });
        let view = build_activity_view("read_file", &args, Some("tool error: file not found"));
        assert_eq!(view.detail.as_deref(), Some("tool error: file not found"));
    }
}
