# fixture full

- 出处：changelogen@0.6.2（generateMarkDown）在合成 git 仓库的真实产出
- 生成：tests/fixtures/changelog-gen.mjs（dev-only）
- 变换：① Breaking 节标题中文化 ② 贡献者节头中文化 ③ 剥除 ungh.cc @username 链接
- 生成配置：hideAuthorEmail: true（本实现默认翻转）；chore(deps) 过滤同原 JS
