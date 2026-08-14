//! 解析层单元测试（镜像 src/cli/parse.rs 与 src/cli/bump.rs 的 overrides /
//! provider 解析）：argv 形态矩阵、overrides 构造、provider 优先级。

use serde_json::json;
use vbumpp_core::cli::{bump_overrides, parse, resolve_provider, BumpArgs, Command};
use vbumpp_core::release::Provider;

use super::argv;

fn parse_bump_args(items: &[&str]) -> BumpArgs {
  match parse(&argv(items)) {
    Ok(Command::Bump(args)) => args,
    other => panic!("应为 Bump，实际 {other:?}"),
  }
}

#[test]
fn help_and_version_flags_win_everywhere() {
  assert!(matches!(parse(&argv(&["--help"])), Ok(Command::Help)));
  assert!(matches!(parse(&argv(&["-h"])), Ok(Command::Help)));
  assert!(matches!(
    parse(&argv(&["foo", "--help"])),
    Ok(Command::Help)
  ));
  assert!(matches!(
    parse(&argv(&["token", "list", "-h"])),
    Ok(Command::Help)
  ));
  assert!(matches!(parse(&argv(&["--version"])), Ok(Command::Version)));
  assert!(matches!(parse(&argv(&["-v"])), Ok(Command::Version)));
  // `--` 之后不再识别 flag
  assert!(matches!(
    parse(&argv(&["--", "--help"])),
    Ok(Command::Bump(_))
  ));
}

#[test]
fn token_routes_with_remaining_args() {
  match parse(&argv(&["token", "list"])) {
    Ok(Command::Token(args)) => assert_eq!(args, vec!["list".to_string()]),
    other => panic!("应为 Token，实际 {other:?}"),
  }
}

#[test]
fn bump_parses_positionals_and_flags() {
  let args = parse_bump_args(&["a.json", "b.json", "-r", "--output", "OUT.md"]);
  assert_eq!(args.files, vec!["a.json".to_string(), "b.json".to_string()]);
  assert!(args.recursive);
  assert_eq!(args.output, "OUT.md");
}

#[test]
fn bump_defaults() {
  let args = parse_bump_args(&[]);
  assert!(args.files.is_empty());
  assert!(!args.recursive);
  assert_eq!(args.output, "CHANGELOG.md");
}

#[test]
fn bump_output_equals_forms() {
  assert_eq!(parse_bump_args(&["--output=OUT.md"]).output, "OUT.md");
  assert_eq!(parse_bump_args(&["-o=OUT.md"]).output, "OUT.md");
  assert_eq!(parse_bump_args(&["-oOUT.md"]).output, "OUT.md");
}

#[test]
fn bump_short_flag_cluster() {
  let args = parse_bump_args(&["-ro", "OUT.md"]);
  assert!(args.recursive);
  assert_eq!(args.output, "OUT.md");
}

#[test]
fn bump_double_dash_treats_rest_as_files() {
  let args = parse_bump_args(&["--", "-r", "--output"]);
  assert_eq!(args.files, vec!["-r".to_string(), "--output".to_string()]);
  assert!(!args.recursive);
}

#[test]
fn bump_unknown_flags_error() {
  assert_eq!(
    parse(&argv(&["--wat"])).unwrap_err(),
    "unknown option: --wat".to_string()
  );
  assert_eq!(
    parse(&argv(&["-x"])).unwrap_err(),
    "unknown option: -x".to_string()
  );
}

#[test]
fn bump_missing_output_value_errors() {
  assert_eq!(
    parse(&argv(&["--output"])).unwrap_err(),
    "option --output requires a value".to_string()
  );
  assert_eq!(
    parse(&argv(&["-o"])).unwrap_err(),
    "option -o requires a value".to_string()
  );
}

#[test]
fn overrides_omit_empty_files() {
  // ADR-0013 浅合并语义锚定：空 files 不注入键
  let overrides = bump_overrides(&BumpArgs {
    files: vec![],
    recursive: false,
    output: "CHANGELOG.md".to_string(),
    provider: None,
    dry_run: false,
  });
  assert!(!overrides.contains_key("files"));
  assert_eq!(overrides["recursive"], json!(false));
  assert_eq!(overrides["changelog"], json!({ "output": "CHANGELOG.md" }));
}

#[test]
fn overrides_include_nonempty_files() {
  let overrides = bump_overrides(&BumpArgs {
    files: vec!["a.json".to_string()],
    recursive: true,
    output: "OUT.md".to_string(),
    provider: None,
    dry_run: false,
  });
  assert_eq!(overrides["files"], json!(["a.json"]));
  assert_eq!(overrides["recursive"], json!(true));
  assert_eq!(overrides["changelog"], json!({ "output": "OUT.md" }));
}

#[test]
fn bump_provider_flag_forms() {
  assert_eq!(
    parse_bump_args(&["--provider", "gitee"])
      .provider
      .as_deref(),
    Some("gitee")
  );
  assert_eq!(
    parse_bump_args(&["--provider=github"]).provider.as_deref(),
    Some("github")
  );
  assert_eq!(parse_bump_args(&[]).provider, None);
  assert_eq!(
    parse(&argv(&["--provider"])).unwrap_err(),
    "option --provider requires a value".to_string()
  );
}

#[test]
fn provider_flag_beats_injection() {
  // ADR-0016 优先级锚定：argv flag > 平台变体注入
  let resolved = resolve_provider(Some("gitee"), Some("github")).unwrap();
  assert_eq!(resolved, Some(Provider::Gitee));
  let resolved = resolve_provider(None, Some("github")).unwrap();
  assert_eq!(resolved, Some(Provider::Github));
  assert_eq!(resolve_provider(None, None).unwrap(), None);
  assert!(resolve_provider(Some("wat"), None).is_err());
}

#[test]
fn release_parses_version_and_flags() {
  match parse(&argv(&[
    "release",
    "5.1.0",
    "--provider",
    "gitee",
    "-o",
    "OUT.md",
  ])) {
    Ok(Command::Release(args)) => {
      assert_eq!(args.version, "5.1.0");
      assert_eq!(args.provider.as_deref(), Some("gitee"));
      assert_eq!(args.output, "OUT.md");
    }
    other => panic!("应为 Release，实际 {other:?}"),
  }
  match parse(&argv(&["release", "v5.1.0", "--provider=gitlab"])) {
    Ok(Command::Release(args)) => {
      assert_eq!(args.version, "v5.1.0");
      assert_eq!(args.provider.as_deref(), Some("gitlab"));
      assert_eq!(args.output, "CHANGELOG.md");
    }
    other => panic!("应为 Release，实际 {other:?}"),
  }
}

#[test]
fn release_requires_version() {
  assert_eq!(
    parse(&argv(&["release"])).unwrap_err(),
    "usage: vbumpp release <version> --provider <github|gitlab|gitee|gitcode>".to_string()
  );
  assert_eq!(
    parse(&argv(&["release", "--provider", "gitee"])).unwrap_err(),
    "usage: vbumpp release <version> --provider <github|gitlab|gitee|gitcode>".to_string()
  );
}

#[test]
fn release_rejects_extra_positionals_and_unknown_flags() {
  assert_eq!(
    parse(&argv(&["release", "1.0.0", "2.0.0"])).unwrap_err(),
    "unexpected argument: 2.0.0".to_string()
  );
  assert_eq!(
    parse(&argv(&["release", "1.0.0", "--wat"])).unwrap_err(),
    "unknown option: --wat".to_string()
  );
  assert_eq!(
    parse(&argv(&["release", "1.0.0", "-r"])).unwrap_err(),
    "unknown option: -r".to_string()
  );
}

#[test]
fn bump_parses_dry_run_flag() {
  // 无值 flag：缺省 false；`--dry-run` 与 `=值` 形态（mri truthy 惯例）均开启
  assert!(!parse_bump_args(&[]).dry_run);
  assert!(parse_bump_args(&["--dry-run"]).dry_run);
  assert!(parse_bump_args(&["--dry-run=false"]).dry_run);
  assert!(parse_bump_args(&["a.json", "--dry-run"]).dry_run);
}

#[test]
fn dry_run_overrides_disable_confirm() {
  // Bump? 确认在 dry-run 跳过（零写盘无需二次确认）：经 overrides 注入
  // confirm=false，流水线零预览分支
  let overrides = bump_overrides(&BumpArgs {
    files: vec![],
    recursive: false,
    output: "CHANGELOG.md".to_string(),
    provider: None,
    dry_run: true,
  });
  assert_eq!(overrides["confirm"], json!(false));
  // 非 dry-run 不注入（交互语义与配置四层合并不受影响）
  let overrides = bump_overrides(&BumpArgs {
    files: vec![],
    recursive: false,
    output: "CHANGELOG.md".to_string(),
    provider: None,
    dry_run: false,
  });
  assert!(!overrides.contains_key("confirm"));
}

#[test]
fn release_parses_dry_run_flag() {
  // 无值 flag：缺省 false；`--dry-run` 与 `=值` 形态（mri truthy 惯例）均开启
  match parse(&argv(&["release", "1.0.0"])) {
    Ok(Command::Release(args)) => assert!(!args.dry_run),
    other => panic!("应为 Release，实际 {other:?}"),
  }
  for forms in [
    &["release", "1.0.0", "--dry-run"][..],
    &["release", "1.0.0", "--dry-run=false"][..],
    &["release", "--dry-run", "1.0.0"][..],
  ] {
    match parse(&argv(forms)) {
      Ok(Command::Release(args)) => assert!(args.dry_run, "{forms:?}"),
      other => panic!("应为 Release，实际 {other:?}"),
    }
  }
}
