//! changelog 展示层：DisplayCommit 解析、excludeScopes 过滤
//! （内建 chore(deps) + 用户配置 + breaking 豁免）、markdown 结构、gitmoji、
//! golden fixtures。

use std::collections::HashMap;

use vbumpp_core::commits::{parse_display_commit, ReferenceType};
use vbumpp_core::git::{GitAuthor, RawCommit};

fn raw(message: &str, body: &str) -> RawCommit {
  RawCommit {
    message: message.to_owned(),
    short_hash: "abc1234".to_owned(),
    author: GitAuthor {
      name: "alice".to_owned(),
      email: "alice@example.com".to_owned(),
    },
    body: body.to_owned(),
  }
}

fn no_scope_map() -> HashMap<String, String> {
  HashMap::new()
}

#[test]
fn parse_conventional_with_scope_and_pr_ref() {
  let commit = parse_display_commit(&raw("feat(ui): add button (#123)", ""), &no_scope_map())
    .expect("conventional 应解析");
  assert_eq!(commit.commit_type, "feat");
  assert_eq!(commit.scope, "ui");
  assert_eq!(commit.description, "add button", "PR 引用从描述剥离");
  assert!(!commit.is_breaking);
  let refs: Vec<(ReferenceType, &str)> = commit
    .references
    .iter()
    .map(|r| (r.ref_type, r.value.as_str()))
    .collect();
  assert_eq!(
    refs,
    [
      (ReferenceType::PullRequest, "#123"),
      (ReferenceType::Hash, "abc1234")
    ],
    "PR 在前、hash 恒在末尾"
  );
}

#[test]
fn parse_issue_ref_stays_in_description() {
  let commit = parse_display_commit(&raw("fix: resolve #45 crash", ""), &no_scope_map()).unwrap();
  assert_eq!(
    commit.description, "resolve #45 crash",
    "issue 引用保留在描述中"
  );
  let types: Vec<ReferenceType> = commit.references.iter().map(|r| r.ref_type).collect();
  assert_eq!(types, [ReferenceType::Issue, ReferenceType::Hash]);
}

#[test]
fn parse_breaking_via_bang_and_footer() {
  let bang = parse_display_commit(&raw("feat!: drop api", ""), &no_scope_map()).unwrap();
  assert!(bang.is_breaking, "`!` 标记");
  let footer =
    parse_display_commit(&raw("fix: adjust", "BREAKING CHANGE: y"), &no_scope_map()).unwrap();
  assert!(footer.is_breaking, "footer 大写形态");
  let lower = parse_display_commit(
    &raw("fix: adjust", "note breaking change: y"),
    &no_scope_map(),
  )
  .unwrap();
  assert!(
    lower.is_breaking,
    "changelogen 展示层为 (?i)breaking change: 文本匹配"
  );
}

#[test]
fn parse_applies_scope_map() {
  let scope_map = HashMap::from([("ui".to_owned(), "界面".to_owned())]);
  let commit = parse_display_commit(&raw("feat(ui): add button", ""), &scope_map).unwrap();
  assert_eq!(
    commit.scope, "界面",
    "scopeMap 解析时应用（changelogen 同位）"
  );
  assert_eq!(
    commit.original_scope, "ui",
    "原始 scope 保留（excludeScopes 匹配基准不受改名牵连）"
  );
}

#[test]
fn parse_non_conventional_returns_none() {
  assert!(parse_display_commit(&raw("just a message", ""), &no_scope_map()).is_none());
  assert!(parse_display_commit(&raw("Merge branch 'x'", ""), &no_scope_map()).is_none());
}

#[test]
fn parse_emoji_prefix_consumed() {
  let code = parse_display_commit(&raw(":sparkles: feat: add x", ""), &no_scope_map()).unwrap();
  assert_eq!(code.commit_type, "feat");
  assert_eq!(code.description, "add x");
  let unicode = parse_display_commit(&raw("✨ feat: add y", ""), &no_scope_map()).unwrap();
  assert_eq!(unicode.commit_type, "feat");
}

#[test]
fn gitmoji_convert_replaces_codes_with_trailing_space() {
  use vbumpp_core::changelog::gitmoji::convert_gitmoji;
  assert_eq!(convert_gitmoji(":sparkles: add x"), "✨  add x");
  assert_eq!(convert_gitmoji(":BUG: fix y"), "🐛  fix y", "大小写不敏感");
  assert_eq!(convert_gitmoji("no codes here"), "no codes here");
  assert_eq!(convert_gitmoji(":unknown: stays"), ":unknown: stays");
  assert_eq!(
    convert_gitmoji(":heavy_plus_sign: dep :heavy_minus_sign: dep"),
    "➕  dep ➖  dep",
    "含 + 键名按字面量匹配"
  );
}

// ---------------------------------------------------------------------------
// markdown 结构（generate_markdown / render_changelog）
// ---------------------------------------------------------------------------

use vbumpp_core::changelog::config::{ChangelogConfig, ChangelogTypeEntry};
use vbumpp_core::changelog::markdown::{generate_markdown, ReleaseRange};
use vbumpp_core::changelog::render_changelog;
use vbumpp_core::commits::{CommitReference, DisplayCommit};
use vbumpp_core::git::RepoConfig;

fn test_config() -> ChangelogConfig {
  ChangelogConfig {
    output: "CHANGELOG.md".to_owned(),
    types: [
      ("feat", "🚀 特性"),
      ("fix", "🩹 修复"),
      ("chore", "🏡 框架"),
      ("BreakingChange", "🚨 破坏性改动"),
    ]
    .into_iter()
    .map(|(n, t)| {
      (
        n.to_owned(),
        ChangelogTypeEntry {
          title: t.to_owned(),
          // 对齐内建默认：chore 组排除 deps（原硬编码过滤的迁居形态）
          exclude_scopes: if n == "chore" {
            vec!["deps".to_owned()]
          } else {
            vec![]
          },
        },
      )
    })
    .collect(),
    repo: Some(RepoConfig {
      provider: Some("github".to_owned()),
      domain: Some("github.com".to_owned()),
      repo: Some("owner/repo".to_owned()),
    }),
    scope_map: HashMap::new(),
    no_authors: false,
    hide_author_email: true,
    exclude_authors: vec![],
    tag_body: "v{{newVersion}}".to_owned(),
    commit_message: "chore: update {{output}}".to_owned(),
  }
}

fn display(
  commit_type: &str,
  scope: &str,
  description: &str,
  author: (&str, &str),
) -> DisplayCommit {
  DisplayCommit {
    short_hash: "abc1234".to_owned(),
    author: GitAuthor {
      name: author.0.to_owned(),
      email: author.1.to_owned(),
    },
    commit_type: commit_type.to_owned(),
    scope: scope.to_owned(),
    original_scope: scope.to_owned(),
    description: description.to_owned(),
    is_breaking: false,
    references: vec![CommitReference {
      ref_type: ReferenceType::Hash,
      value: "abc1234".to_owned(),
    }],
  }
}

fn range<'a>() -> ReleaseRange<'a> {
  ReleaseRange {
    from: "v1.0.0",
    to: "v1.1.0",
    new_version: Some("1.1.0"),
  }
}

#[test]
fn markdown_header_and_compare_link() {
  let config = test_config();
  let md = generate_markdown(&[], &config, &range());
  assert!(md.starts_with("## v1.1.0"), "tagBody 渲染 ## 头：{md}");
  assert!(md.contains("[compare changes](https://github.com/owner/repo/compare/v1.0.0...v1.1.0)"));
}

#[test]
fn markdown_header_fallback_without_new_version() {
  let config = test_config();
  let range = ReleaseRange {
    from: "v1.0.0",
    to: "v1.1.0",
    new_version: None,
  };
  let md = generate_markdown(&[], &config, &range);
  assert!(md.starts_with("## v1.0.0...v1.1.0"), "{md}");
}

#[test]
fn markdown_compare_bitbucket_and_absent_repo() {
  let mut config = test_config();
  config.repo = Some(RepoConfig {
    provider: Some("bitbucket".to_owned()),
    domain: Some("bitbucket.org".to_owned()),
    repo: Some("owner/repo".to_owned()),
  });
  let md = generate_markdown(&[], &config, &range());
  assert!(
    md.contains(
      "[compare changes](https://bitbucket.org/owner/repo/branches/compare/v1.1.0%0Dv1.0.0)"
    ),
    "{md}"
  );

  config.repo = None;
  let commit = display("feat", "", "add x", ("alice", "a@e.com"));
  let md = generate_markdown(&[commit], &config, &range());
  assert!(
    !md.contains("compare changes"),
    "无 repo 不出 compare 行：{md}"
  );
  assert!(md.contains("Add x (abc1234)"), "无 repo 引用为纯文本：{md}");
}

#[test]
fn markdown_groups_in_declaration_order_with_reverse_and_empty_skip() {
  let config = test_config();
  let commits = vec![
    display("fix", "", "second fix", ("alice", "a@e.com")),
    display("feat", "ui", "first feat", ("bob", "b@e.com")),
    display("fix", "", "first fix", ("alice", "a@e.com")),
  ];
  let md = generate_markdown(&commits, &config, &range());
  let feat_pos = md.find("### 🚀 特性").unwrap();
  let fix_pos = md.find("### 🩹 修复").unwrap();
  assert!(feat_pos < fix_pos, "声明序 feat 在前");
  assert!(!md.contains("🏡 框架"), "空组跳过");
  let first_fix = md.find("First fix").unwrap();
  let second_fix = md.find("Second fix").unwrap();
  assert!(first_fix < second_fix, "组内 reverse（旧→新）");
  assert!(
    md.contains("- **ui:** First feat"),
    "scope 加粗 + upperFirst"
  );
}

#[test]
fn markdown_breaking_section_with_custom_title_and_marker() {
  let config = test_config();
  let mut breaking = display("feat", "", "drop api", ("alice", "a@e.com"));
  breaking.is_breaking = true;
  breaking.references = vec![
    CommitReference {
      ref_type: ReferenceType::PullRequest,
      value: "#9".to_owned(),
    },
    CommitReference {
      ref_type: ReferenceType::Hash,
      value: "abc1234".to_owned(),
    },
  ];
  let md = generate_markdown(&[breaking], &config, &range());
  assert!(md.contains("#### 🚨 破坏性改动"), "中文节标题直生：{md}");
  assert!(md.contains("- ⚠️  Drop api ([#9](https://github.com/owner/repo/pull/9))"));
  let occurrences = md.matches("Drop api").count();
  assert_eq!(occurrences, 2, "breaking 行在分组与 breaking 节各出现一次");
}

#[test]
fn markdown_breaking_title_fallback_when_disabled() {
  let mut config = test_config();
  config.types.retain(|(n, _)| n != "BreakingChange");
  let mut breaking = display("feat", "", "drop api", ("alice", "a@e.com"));
  breaking.is_breaking = true;
  let md = generate_markdown(&[breaking], &config, &range());
  assert!(
    md.contains("#### ⚠️ Breaking Changes"),
    "禁用后回退英文默认（对齐原 JS hack 回退链）：{md}"
  );
}

#[test]
fn markdown_contributors_default_hides_email() {
  let config = test_config();
  let commits = vec![
    display("feat", "", "add x", ("alice", "alice@example.com")),
    display("fix", "", "fix y", ("alice", "alice@example.com")),
  ];
  let md = generate_markdown(&commits, &config, &range());
  assert!(md.contains("### ❤️ Contributors"), "{md}");
  assert!(
    md.ends_with("- Alice"),
    "默认隐邮箱且 formatName 规范化：{md}"
  );
  assert!(!md.contains("alice@example.com"));
  assert_eq!(md.matches("- Alice").count(), 1, "按名去重");
}

#[test]
fn markdown_contributors_email_shown_when_not_hidden() {
  let mut config = test_config();
  config.hide_author_email = false;
  let commits = vec![
    display("feat", "", "add x", ("alice", "alice@example.com")),
    display(
      "fix",
      "",
      "fix y",
      ("bob", "12345+bob@users.noreply.github.com"),
    ),
  ];
  let md = generate_markdown(&commits, &config, &range());
  assert!(md.contains("- Alice <alice@example.com>"), "{md}");
  assert!(md.ends_with("- Bob"), "noreply 邮箱不外显：{md}");
}

#[test]
fn markdown_contributors_exclude_bot_and_noauthors() {
  let mut config = test_config();
  config.exclude_authors = vec!["evil".to_owned()];
  let commits = vec![
    display("feat", "", "add x", ("dependabot[bot]", "bot@e.com")),
    display("fix", "", "fix y", ("evilone", "evil@e.com")),
    display("docs", "", "doc z", ("carol", "c@e.com")),
  ];
  // docs 不在 test_config 的 types 里 → 被 render 过滤；改用 generate 直接测贡献者收集
  let md = generate_markdown(&commits, &config, &range());
  assert!(md.contains("- Carol"));
  assert!(!md.contains("bot@e.com"), "[bot] 跳过");
  assert!(!md.contains("evil@e.com"), "excludeAuthors 子串匹配");

  config.no_authors = true;
  let md = generate_markdown(&commits, &config, &range());
  assert!(!md.contains("Contributors"), "noAuthors 整节不出");
}

#[test]
fn render_filters_unknown_type_and_chore_deps() {
  let config = test_config();
  let raws = vec![
    raw("feat: add x", ""),
    raw("chore(deps): bump serde", ""),
    raw("chore(deps)!: bump serde major", ""),
    raw("chore: tidy", ""),
    raw("docs: write readme", ""),
    raw("not conventional", ""),
  ];
  let md = render_changelog(&raws, &config, &range());
  assert!(md.contains("Add x"));
  assert!(!md.contains("bump serde\n"), "chore(deps) 非 breaking 滤除");
  assert!(
    md.contains("⚠️  Bump serde major"),
    "chore(deps) breaking 豁免保留"
  );
  assert!(md.contains("Tidy"), "chore 非 deps 保留");
  assert!(!md.contains("readme"), "未知类型（docs 不在 types 表）滤除");
  assert!(!md.contains("not conventional"), "非 conventional 滤除");
}

#[test]
fn render_exclude_scopes_user_config_with_breaking_exemption() {
  // 用户配置的 scope 级排除：docs(agent) 不进 changelog，同组其余照常；
  // 命中排除 scope 的 breaking 提交一律豁免照常显示
  let mut config = test_config();
  config.types.push((
    "docs".to_owned(),
    ChangelogTypeEntry {
      title: "📖 文档".to_owned(),
      exclude_scopes: vec!["agent".to_owned()],
    },
  ));
  let raws = vec![
    raw("docs(agent): update AGENTS.md", ""),
    raw("docs(agent)!: rewrite agents contract", ""),
    raw("docs: write guide", ""),
    raw("chore(agent): tweak config", ""),
  ];
  let md = render_changelog(&raws, &config, &range());
  assert!(md.contains("Write guide"), "同组未命中提交照常显示：{md}");
  assert!(
    !md.contains("Update AGENTS.md"),
    "docs(agent) 非 breaking 滤除：{md}"
  );
  assert!(
    md.contains("⚠️  Rewrite agents contract"),
    "docs(agent)! breaking 豁免照常显示：{md}"
  );
  assert!(
    md.contains("Tweak config"),
    "chore 组未配 agent 排除、不受 docs 规则牵连：{md}"
  );
}

#[test]
fn render_exclude_scopes_match_original_scope_not_scope_map() {
  // 匹配基准为提交原始 scope：scopeMap 把 agent 改名 ai 后，排除规则
  // 仍按 "agent" 命中；显示名照旧用 scopeMap 后的值
  let mut config = test_config();
  config.scope_map = HashMap::from([("agent".to_owned(), "ai".to_owned())]);
  config.types.push((
    "docs".to_owned(),
    ChangelogTypeEntry {
      title: "📖 文档".to_owned(),
      exclude_scopes: vec!["agent".to_owned()],
    },
  ));
  let raws = vec![
    raw("docs(agent): hidden note", ""),
    raw("docs(agent)!: visible note", ""),
  ];
  let md = render_changelog(&raws, &config, &range());
  assert!(!md.contains("Hidden note"), "原始 scope 命中排除：{md}");
  assert!(
    md.contains("**ai:** ⚠️  Visible note"),
    "豁免提交显示名仍用 scopeMap 后值：{md}"
  );
}

#[test]
fn render_exclude_scopes_case_sensitive() {
  let mut config = test_config();
  config.types.push((
    "docs".to_owned(),
    ChangelogTypeEntry {
      title: "📖 文档".to_owned(),
      exclude_scopes: vec!["Agent".to_owned()],
    },
  ));
  let raws = vec![raw("docs(agent): keep me", "")];
  let md = render_changelog(&raws, &config, &range());
  assert!(
    md.contains("Keep me"),
    "大小写敏感：Agent 不命中 agent：{md}"
  );
}

#[test]
fn render_exclude_scopes_empty_array_disables_builtin_deps_filter() {
  // `[]` 为显式关闭内建 deps 过滤的出口：chore(deps) 恢复显示
  let mut config = test_config();
  for (name, entry) in &mut config.types {
    if name == "chore" {
      entry.exclude_scopes = vec![];
    }
  }
  let raws = vec![raw("chore(deps): bump serde", "")];
  let md = render_changelog(&raws, &config, &range());
  assert!(md.contains("Bump serde"), "空数组关闭内建过滤：{md}");
}

#[test]
fn render_applies_gitmoji_and_uppercase() {
  let config = test_config();
  let raws = vec![raw("feat: add :sparkles: x", "")];
  let md = render_changelog(&raws, &config, &range());
  assert!(md.contains("Add ✨  x"), "gitmoji 正文转换：{md}");
}

// ---------------------------------------------------------------------------
// golden fixtures：真 changelogen 0.6.2 产出经申报偏差变换（见 NOTES.md 与
// tests/fixtures/changelog-gen.ts 头注释；节标题中文化偏差已随英文默认移除），
// 逐字节比对
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoldenInput {
  raw_commits: Vec<RawCommit>,
  from: String,
  to: String,
  new_version: String,
  repo: RepoConfig,
}

fn golden(name: &str) {
  use vbumpp_core::changelog::config::resolve_changelog_config;
  let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("tests/fixtures/changelog")
    .join(name);
  let input: GoldenInput =
    serde_json::from_str(&std::fs::read_to_string(dir.join("input.json")).unwrap()).unwrap();
  let expected = std::fs::read_to_string(dir.join("expected.md")).unwrap();
  // 内建默认解析 + fixture 注入 repo（默认 repo 为 None——线上由 git remote 解析补位）
  let mut config = resolve_changelog_config(None, None).unwrap();
  config.repo = Some(input.repo);
  let range = ReleaseRange {
    from: &input.from,
    to: &input.to,
    new_version: Some(&input.new_version),
  };
  let actual = render_changelog(&input.raw_commits, &config, &range);
  assert_eq!(actual, expected, "golden fixture {name} 逐字节比对");
}

#[test]
fn golden_full_matches_changelogen_output() {
  golden("full");
}

#[test]
fn golden_minimal_matches_changelogen_output() {
  golden("minimal");
}

#[test]
fn markdown_compare_links_any_provider_upstream_truth() {
  // 上游 formatCompareChanges 对任意 provider 恒出链接（仅 bitbucket 特判路径）
  let mut config = test_config();
  config.repo = Some(RepoConfig {
    provider: Some("gitee".to_owned()),
    domain: Some("gitee.com".to_owned()),
    repo: Some("owner/repo".to_owned()),
  });
  let md = generate_markdown(&[], &config, &range());
  assert!(
    md.contains("[compare changes](https://gitee.com/owner/repo/compare/v1.0.0...v1.1.0)"),
    "{md}"
  );
}

#[test]
fn markdown_reference_raw_text_for_unsupported_provider() {
  let mut config = test_config();
  config.repo = Some(RepoConfig {
    provider: Some("gitee".to_owned()),
    domain: Some("gitee.com".to_owned()),
    repo: Some("owner/repo".to_owned()),
  });
  let commit = display("feat", "", "add x", ("alice", "a@e.com"));
  let md = generate_markdown(&[commit], &config, &range());
  assert!(
    md.contains("Add x (abc1234)"),
    "非支持 provider 引用纯文本：{md}"
  );
}

#[test]
fn markdown_empty_new_version_falls_back_like_falsy() {
  let config = test_config();
  let range = ReleaseRange {
    from: "v1.0.0",
    to: "v1.1.0",
    new_version: Some(""),
  };
  let md = generate_markdown(&[], &config, &range);
  assert!(
    md.starts_with("## v1.0.0...v1.1.0"),
    "空串按上游 falsy 回落：{md}"
  );
}
