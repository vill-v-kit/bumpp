// 本文件由 website/scripts/capture-home-demo.sh 生成，勿手改。
// 内容：target/release/vbumpp 在临时 fixture 中的真实交互发版输出（菜单选定 minor），
// pty 原始字节流经 terminal-screen.mjs 塌缩为最终屏幕，绝对路径已洗白为 ~。
// 首行 `$ vbumpp` 为演示提示符（非捕获内容）；复跑脚本可字节级复现其余内容。
export const DEMO_TERMINAL = `$ vbumpp
✔ Current version 1.0.0 ·         minor 1.1.0
[main e11eadf] chore: update CHANGELOG.md
 1 file changed, 19 insertions(+)
 create mode 100644 CHANGELOG.md
✔ Update CHANGELOG.md success
✔ Updated ~/my-project/package.json to 1.1.0
[main 9995c2b] chore: release v1.1.0
 1 file changed, 1 insertion(+), 1 deletion(-)
ℹ Git commit
ℹ Git tag
To ../remote.git
   ba44e61..9995c2b  main -> main
To ../remote.git
 * [new tag]         v1.1.0 -> v1.1.0
✔ Git push
`;
