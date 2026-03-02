//! Hook installation tests.

use crate::commands::install::acd_hook_definitions;

#[test]
fn test_acd_hook_definitions_has_nine_entries() {
    let defs = acd_hook_definitions();
    // 9 hooks: PostToolUse removed for experiment (acd-ws6), PreCompact added (acd-wdaj),
    // PreToolUse(AskUserQuestion) added for question status (acd-6sjw)
    assert_eq!(
        defs.len(),
        9,
        "should define 9 hooks (PostToolUse removed for experiment, PreCompact added, \
         PreToolUse(AskUserQuestion) added)"
    );
}

#[test]
fn test_acd_hook_definitions_all_use_acd_command() {
    let defs = acd_hook_definitions();
    for (_, command, _) in &defs {
        assert!(
            command.starts_with("acd claude-hook "),
            "hook command should start with 'acd claude-hook': {}",
            command
        );
    }
}

#[test]
fn test_acd_hook_definitions_notification_hooks_have_matchers() {
    let defs = acd_hook_definitions();
    let notification_hooks: Vec<_> = defs
        .iter()
        .filter(|(event, _, _)| *event == claude_hooks::HookEvent::Notification)
        .collect();
    assert_eq!(
        notification_hooks.len(),
        2,
        "should have 2 Notification hooks"
    );
    for (_, _, matcher) in &notification_hooks {
        assert!(matcher.is_some(), "Notification hooks must have a matcher");
    }
}

#[test]
fn test_acd_hook_definitions_includes_pre_tool_use() {
    let defs = acd_hook_definitions();
    let has_pre_tool_use = defs
        .iter()
        .any(|(event, _, _)| *event == claude_hooks::HookEvent::PreToolUse);
    assert!(has_pre_tool_use, "should have PreToolUse hook");
    // PostToolUse removed for experiment (acd-ws6)
    let has_post_tool_use = defs
        .iter()
        .any(|(event, _, _)| *event == claude_hooks::HookEvent::PostToolUse);
    assert!(
        !has_post_tool_use,
        "PostToolUse should be absent (acd-ws6 experiment)"
    );
}

#[test]
fn test_acd_hook_definitions_pre_tool_use_ask_user_question() {
    let defs = acd_hook_definitions();
    // Find the PreToolUse hook with AskUserQuestion matcher
    let ask_user_question_hook = defs.iter().find(|(event, command, matcher)| {
        *event == claude_hooks::HookEvent::PreToolUse
            && *command == "acd claude-hook question"
            && matcher.as_deref() == Some("AskUserQuestion")
    });
    assert!(
        ask_user_question_hook.is_some(),
        "should have PreToolUse(AskUserQuestion) hook that calls 'acd claude-hook question'"
    );

    // Verify the general PreToolUse(working) hook is also present
    let pre_tool_use_working = defs.iter().find(|(event, command, matcher)| {
        *event == claude_hooks::HookEvent::PreToolUse
            && *command == "acd claude-hook working"
            && matcher.is_none()
    });
    assert!(
        pre_tool_use_working.is_some(),
        "should still have PreToolUse (no matcher) hook that calls 'acd claude-hook working'"
    );
}
