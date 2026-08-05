# fixture minimal

- 出处：changelogen@0.6.2（generateMarkDown）在合成 git 仓库的真实产出
- 生成：tests/fixtures/changelog-gen.mjs（dev-only）
- 变换：剥除 ungh.cc @username 链接（原 ① 节标题中文化 ② 贡献者节头中文化
  随 ADR-0017 英文默认移除，fixture 回到 changelogen 原生英文产出）
- 生成配置：hideAuthorEmail: true（本实现默认翻转）；chore(deps) 过滤同原 JS
