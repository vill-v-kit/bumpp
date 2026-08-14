# Token host 作用域键与 remove 交互矩阵:provider@host 复合键、删除护栏、手写解析器维持

多个私有 GitLab 实例并存的用户需要按实例各存一份 token;token 管理面随之扩出一组新 flag(`--host` / `--all` / `--yes` / `--dry-run`)。本 ADR 记录三件事:存储内的 **host 作用域键方案**(含旧键保留的兼容性理由)、`token remove` 的**交互矩阵**(删除护栏的完整口径)、flag 增长下**维持手写解析器**的评估结论。

## Decisions

### `provider@host` 复合键

- 存储内部 JSON map 在 provider 级键(`gitlab` 等)之外支持 host 作用域复合键 `provider@host`(如 `gitlab@https://gitlab-a.com`)。VBTK v1 加密信封与 JS 时代字节兼容不动、旧键零迁移保留——信封格式与 map 形状都不变,只是多了一种键名约定。
- host 统一经一个规范化函数键化:无 scheme 自动补 `https://`(显式 `http://` 原样保留,覆盖内网纯 HTTP 实例)、scheme/host 小写、去尾斜杠、保留端口与路径(兼容 GitLab relative-url-root 部署)。写入(`token set --host`)与读取(release 解析链)共用同一函数——两侧归一一致才能保证宽松写法相撞到同一键。
- **provider 级旧键保留且解析链必须回落**(`gitlab@<有效 host>` 精确键 → `gitlab` provider 级键 → `GITLAB_TOKEN` → 报错)。这是向后兼容硬要求:存量自建 GitLab 用户的 token 都存在 `gitlab` 键下,不落回则他们的现有配置全部失效。
- `--host` 目前只开放给 gitlab:键格式虽 provider 无关,但其他 provider 没有 host 配置通路,存进去永远不会被读到——set 与 remove 同一门禁拒绝(未来 GHE 支持时一并解除)。

### remove 交互矩阵

- 目标选择四形态:`remove gitlab`(仅 provider 级键,精确项语义不触碰 `gitlab@*`)、`remove gitlab --host <url>`(仅该 host 键)、`remove gitlab --all`(provider 级键 + 全部 host 键,自定义 host 多了一键清)、`remove --all`(清空存储)。
- `--dry-run` 优先级最高:只打印将删清单(友好形态 `gitlab (https://...)`),不确认、不删除、exit 0;与 `--yes` 同给也只打印。
- 无 `--yes` 且无 `--dry-run`:先列清单再二次确认(dialoguer Confirm,**默认 No**——安全方向;Esc/Ctrl+C 视为拒绝);拒绝则 `canceled` 警告、exit 0。
- 非 TTY 环境且无 `--yes`:无法交互确认即报错引导 `--yes`、exit 1——不能静默删;CI 用 `--yes` 显式授权。
- 目标不存在沿用 warn `no token found for ...` + exit 0;`--all` 形态只删实际存在的条目并如实列清单,全无匹配则 warn。
- 确认要求只作用于 remove:`token set`(覆盖语义)与 `token list` 行为不变。

### 手写解析器维持,不引入 clap

- flag 已增长到 token 子命令四个(`--host` / `--all` / `--yes` / `--dry-run`),仍维持手写解析:JS 时代 cac/mri 的 argv 语义(布尔 flag 带值 truthy、短 flag 簇合并、`--` 位置参数分隔、`--flag=value` 等值形态)已由手写解析器逐条编码,并有全套 CLI 测试与文档以其为 parity 基准。
- 解析重复经 token 子命令的 flag 扫描小 helper 收口(认 `--flag` / `--flag=value` / `--` 分隔,未知 flag 报错),各 action 只声明自己的 flag 名单——flag 增量的边际成本已经很低,不构成换解析器的理由。

## Alternatives considered

- **嵌套结构存储(被否决)**:map 值改为 `{"gitlab": {"default": "...", "hosts": {"https://...": "..."}}}` 这类层级。被否决:改变了 map 形状,JS 时代的旧 CLI 按 `tokens[provider]` 读到的是对象而非字符串——同一份存储的双向兼容就此打破,违背「VBTK v1 与 JS 时代字节兼容不动、旧键零迁移」的前置共识;扁平复合键在不动任何既有字节的前提下表达同样的信息。
- **按 host 分文件(被否决)**:每个实例一个存储文件(如 `tokens/<encoded-host>.bin`)。被否决:密钥 `key.bin` 与单文件原子写语义都是围绕「一个存储文件」建立的,分文件要重新回答密钥共享、批量列出、清空空文件清理等一系列一致性问题;list/remove --all 还要跨文件聚合,复杂度远高于一个键名约定。
- **remove 引入 clap(被否决)**:借 flag 增长换成 derive 解析器。被否决:换解析器等于重写全部 argv 边界语义(mri truthy、短簇、值吞并规则)与全部报错文案,既有 CLI 测试矩阵与文档的对齐成本巨大,而用户可见收益为零;cac parity 是本工具对 JS 时代用户的兼容承诺,不是可随意替换的实现细节。未来 CLI 面持续膨胀(更多子命令、嵌套参数)时再重启此议。

## Consequences

- 多个私有 GitLab 实例与 gitlab.com 可并存:每仓库 `.vbumpprc.*` 配各自 `gitlab.host`,token 逐 host 录入,解析链按配置 host 精确命中;存量单实例用户零感知(provider 级键回落)。
- 宽松 host 写法不会写出重复条目:`gitlab-a.com` / `https://gitlab-a.com/` / `HTTPS://GitLab-A.com` 在 set、list、remove、release 四侧归一到同一键。
- 删除操作有了护栏:默认确认的拒绝方向是安全的;非 TTY 不会静默删;`--dry-run` 提供零风险的事前核对;`--yes` 保留 CI 自动化通路。
- 解析器维持手写:argv 语义与报错文案的 parity 基准不动;flag 增量只改各 action 的 flag 名单。
