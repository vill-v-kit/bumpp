//! 集成测试共享辅助。
#![allow(dead_code)] // 各测试二进制按需取用——逐二进制编译必有未用项

use std::path::Path;
use std::process::Command;
use std::sync::Once;

static ISOLATE: Once = Once::new();

/// 全部 provider token 环境变量名（env 净化用例共用——新增 provider 时
/// 唯一维护点，与生产 `Provider::env_vars()` 对应）
pub const PROVIDER_TOKEN_ENV_VARS: &[&str] = &[
  "GH_TOKEN",
  "GITHUB_TOKEN",
  "GITLAB_TOKEN",
  "GITEE_TOKEN",
  "GITCODE_TOKEN",
];

/// 把全局配置目录指向不存在的路径，使走 `read_document`（env 解析全局层）的
/// 测试不受宿主机真实 `~/.vbumpp/config.*` 影响。进程内所有线程 set 同值，
/// Once 保证单次执行，无并发竞态。
pub fn isolate_global_home() {
  ISOLATE.call_once(|| {
    std::env::set_var("VBUMPP_HOME", "/nonexistent/vbumpp-global-home");
  });
}

/// 临时 git 仓库命令执行：断言成功并返回 trim 后的 stdout
pub fn git(dir: &Path, args: &[&str]) -> String {
  let output = Command::new("git")
    .args(args)
    .current_dir(dir)
    .output()
    .unwrap();
  assert!(
    output.status.success(),
    "git {args:?} 失败：{}",
    String::from_utf8_lossy(&output.stderr)
  );
  String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// bump 编排 fixture（COL-60）：package.json 1.0.0 + tag v1.0.0 + 一个 feat
/// commit（供 changelog）；`config` 原文写入 .vbumpprc.toml。remote 仅供
/// changelog repo 推断（纯本地，无任何网络动作）；push 由各用例配置关闭
pub fn init_bump_repo(dir: &tempfile::TempDir, config: &str) -> std::path::PathBuf {
  let path = dir.path().to_path_buf();
  git(&path, &["init", "-b", "main"]);
  git(&path, &["config", "user.email", "test@example.com"]);
  git(&path, &["config", "user.name", "Test"]);
  git(&path, &["config", "commit.gpgsign", "false"]);
  git(&path, &["config", "tag.gpgsign", "false"]);
  std::fs::write(
    path.join("package.json"),
    "{\n  \"version\": \"1.0.0\"\n}\n",
  )
  .unwrap();
  std::fs::write(path.join(".vbumpprc.toml"), config).unwrap();
  git(&path, &["add", "."]);
  git(&path, &["commit", "-m", "chore: init"]);
  git(&path, &["tag", "v1.0.0"]);
  git(
    &path,
    &["remote", "add", "origin", "git@github.com:owner/repo.git"],
  );
  std::fs::write(path.join("feat.txt"), "x").unwrap();
  git(&path, &["add", "."]);
  git(&path, &["commit", "-m", "feat: new thing"]);
  path
}
