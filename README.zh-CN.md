# IWE：你与 AI 智能体共同使用的 Markdown 知识库

[English](README.md) · **简体中文** · [한국어](README.ko.md)

> 你的笔记就是智能体的记忆。IWE 是你们一起打磨它们的地方。

**[iwe.md](https://iwe.md/?ref=readme-header)** · **[下载 Mac 版](https://iwe.md/app/?ref=readme-header)** · [文档](https://iwe.md/docs/?ref=readme-header) · [Reddit](https://www.reddit.com/r/iwe/)

[![Crates.io](https://img.shields.io/crates/v/iwe.svg)](https://crates.io/crates/iwe)
[![License](https://img.shields.io/crates/l/iwe.svg)](https://github.com/iwe-org/iwe/blob/master/LICENSE-APACHE)
[![Documentation](https://img.shields.io/badge/docs-iwe.md-blue)](https://iwe.md)

太多与 AI 的对话都是从零开始。你解释你的项目、你的决策、你的思路——到了下一次会话，又得从头再解释一遍。而你自己的笔记放在别处，智能体根本够不着。

更糟的是，智能体在它自己的内部记忆里“记住”了某些东西，但你很难看到它存了什么，也没法纠正它记错的地方。而且这份记忆不会跟着你去到你使用的其他智能体那里。

IWE 为你和你的 AI 智能体提供**一个共享的工作空间，就在你的项目里**：一个由相互链接的 Markdown 文档组成的文件夹——规格说明、计划、决策等等。把你自己的想法放在这里，让智能体保存你们一起学到的东西，并随着工作推进随时回到这些上下文。你可以自己阅读、编辑和整理这些文档——它们始终属于你。

## 选择你的工作方式

使用 IWE 有四种方式，可以按自己的习惯自由组合：

| 工具 | 使用方式 |
| --- | --- |
| **[iWe for Mac](https://iwe.md/?mode=app&ref=readme-table)** | 在原生应用中写作和浏览，并连接你现有的 Claude/Codex 智能体。 |
| **[你自己的编辑器](https://iwe.md/?mode=editor&ref=readme-table)** | 继续使用 Neovim、Helix、VS Code 或 Zed，并获得笔记跳转、反向链接和自动补全。 |
| **[CLI](https://iwe.md/docs/cli/?ref=readme-table)** | 在终端或脚本中搜索和管理笔记。 |
| **[Claude/Codex](https://iwe.md/docs/agentic/?ref=readme-table)** | 在对话中让智能体读取、写入和整理记忆。 |

这四种方式都作用于同一批 Markdown 文档。在编辑器里起草规格说明，和 Claude/Codex 一起完善它，在 iWe 应用中浏览，或者用 CLI 搜索。随意组合即可，每个工具也都可以单独使用。

**自带你的 AI。** IWE CLI 和 iWe Mac 应用都不内置 AI 模型，它们本身就是独立的笔记工具。需要 AI 帮助时，使用你现有的 Claude/Codex 智能体——在终端中或直接在 Mac 应用里。

<a href="https://iwe.md/?ref=readme-shot"><img src="https://iwe.md/images/app/GH-screen-1.1.png" alt="iWe for Mac 展示项目计划和相互链接的文档" width="820"></a>

<p>
  <a href="https://iwe.md/?ref=readme-shot"><img src="https://iwe.md/images/app/GH-screen-2.png" alt="带图表的缺陷跟踪" width="32%"></a>
  <a href="https://iwe.md/?ref=readme-shot"><img src="https://iwe.md/images/app/GH-screen-3.png" alt="包含代码的技术文档" width="32%"></a>
  <a href="https://iwe.md/?ref=readme-shot"><img src="https://iwe.md/images/app/GH-screen-4.png" alt="发布文档" width="32%"></a>
</p>

*计划 · 图表 · 技术文档 · 发布——点击截图，在 iwe.md 上查看 iWe 应用的实际效果。*

## 为什么选择 IWE

- **共享上下文。** 智能体可以使用你为自己写的同一批笔记。笔记之间相互链接，一篇笔记可以同时属于多个主题，因此你可以顺着一个想法追踪到它关联的任何地方。
- **看得见的记忆。** 智能体保存的决策和经验都是普通笔记，你可以浏览、修改或删除。
- **笔记始终属于你。** 一切都是你机器上某个文件夹里的 Markdown。你可以备份、同步，或放进 git。保存笔记不需要 IWE 云账号；连接 AI 智能体时，需遵循该智能体自身的设置和账号要求。

**[在 iwe.md 上看看实际效果 →](https://iwe.md/?ref=readme-why)**

## iWe for Mac

如果你想要一个专门存放笔记的空间，可以使用原生 Mac 应用。打开一个笔记文件夹，就能获得一个快速、键盘友好的写作和浏览空间。你还可以连接现有的 Claude/Codex 智能体，与你并肩工作。

- **与你的 Claude/Codex 智能体协作。** 连接你已经在用的智能体，就你的笔记提问，让它起草或重新整理笔记。它工作时，你能看到它的修改实时出现在文档中。
- **放心尝试。** 每次智能体运行都是一个撤销步骤——尝试一个想法，不满意就回滚。
- **追随你的想法。** 在相关笔记之间跳转，几次按键即可搜索，并直接在文档中查看 Mermaid 图表。你的编辑器和其他工具也可以继续处理同样的文件。

**[在 iwe.md 上了解 iWe for Mac →](https://iwe.md/?ref=readme-download)**

打开一个笔记文件夹，从一篇关于你手头工作的笔记开始。连接 Claude/Codex 之后，试着让它总结这篇笔记，或帮你展开一个想法。

## 你自己的编辑器

继续在 [Neovim](https://iwe.md/docs/editors/neovim/)、[Helix](https://iwe.md/docs/editors/helix/)、[VS Code](https://iwe.md/docs/editors/vscode/) 或 [Zed](https://iwe.md/docs/editors/zed/) 中写作。IWE 为你的笔记提供链接自动补全、反向链接、搜索和安全重命名。

按照对应编辑器的指南操作，打开你的笔记文件夹，试着用自动补全把两篇笔记链接起来。即使不连接 AI 智能体，你也可以用 IWE 进行自己的写作。

## CLI

更喜欢终端？用 IWE 搜索、创建和重新组织笔记，或把它集成到你的脚本中。CLI 不内置 AI：这些命令可以独立运行。你现有的 Claude/Codex 智能体也可以使用 IWE 的工具。

通过 Homebrew 安装，或在[安装指南](https://iwe.md/docs/getting-started/installation/)中选择其他方式：

```bash
brew install iwe-org/iwe/iwe
```

然后在你的笔记文件夹中运行以下命令：

```bash
iwe init
iwe find
iwe find --lexical "project decisions"
```

`iwe find` 列出你的笔记；`--lexical` 搜索笔记内容。更多内容请参阅 [CLI 参考](https://iwe.md/docs/cli/)和[工作流示例](docs/cli-workflows.md)。

## Claude/Codex

你可以完全通过智能体来使用 IWE。让 Claude/Codex 保存决策、查找过去的工作，并整理你学到的东西。智能体通过 IWE 处理你的笔记，而你随时可以自己打开这些文件。

IWE 不绑定于某一个 AI。同一批笔记可用于 Claude Code、Claude Desktop、Codex、Gemini、Cursor，以及任何支持 [Model Context Protocol](https://modelcontextprotocol.io) 的工具。

**给 Claude/Codex 一份记忆** —— 对于 Claude Code，安装插件后，在任何你希望它记住的项目中运行 `/iwe:init`。对于 Codex，请按照下面的智能体连接步骤操作。

```text
/plugin marketplace add iwe-org/skills
/plugin install iwe@iwe-org
/iwe:init
```

然后让它保存一条你当前工作中的决策，并在项目里打开它创建的那篇笔记。

**连接其他智能体** —— 在已安装 Homebrew 的情况下，安装 IWE：

```bash
brew install iwe-org/iwe/iwe
```

在你的笔记文件夹中运行 `iwe init`，然后按照**[智能体连接指南](https://iwe.md/docs/agentic/)**添加 `iwec` MCP 服务器，并将该文件夹设为其工作目录。其他安装方式请参阅[安装指南](https://iwe.md/docs/getting-started/installation/)。

连接完成后，问问你的智能体：“浏览我的 IWE 笔记，并总结里面有什么。”

或者让智能体自己完成设置——把下面这段粘贴给任何能运行命令的智能体：

```text
为我的笔记设置 IWE：安装它（brew install iwe-org/iwe/iwe、
npm install -g @iwe-org/iwe 或 cargo install iwe iwes iwec），在我的笔记目录中
运行 `iwe init`，然后添加 `iwec` MCP 服务器，并将其工作目录设为该文件夹。
文档：https://iwe.md/docs/agentic/
```

## 了解更多

- [入门](https://iwe.md/docs/getting-started/installation/) —— 安装与设置
- [与 AI 协作](https://iwe.md/docs/agentic/) —— 连接你的智能体
- [使用指南](https://iwe.md/docs/getting-started/usage/) —— 日常工作流
- [示例](https://iwe.md/docs/examples/) —— 大家如何使用 IWE
- [CLI 参考](https://iwe.md/docs/cli/)和[MCP 服务器](https://iwe.md/docs/agentic/mcp/) —— 写给对技术细节感兴趣的你

**现成的工作空间：** [dev-workspace](https://github.com/iwe-org/dev-workspace) —— 为编程智能体准备的项目记忆 · [marketing-workspace](https://github.com/iwe-org/marketing-workspace) —— 为营销智能体准备的活动记忆。

## 参与进来

正在用自己的笔记试用 IWE？告诉我们哪些地方有用、哪些地方卡住了。欢迎在 [Discussions](https://github.com/iwe-org/iwe/discussions) 中提问、反馈不完善之处，或分享你的使用案例。

IWE 是开源项目。你也可以报告 [bug](https://github.com/iwe-org/iwe/issues)，或帮助改进[文档](docs/)。

**社区：** [Twitter/X](https://x.com/iwe_md) · [Reddit](https://www.reddit.com/r/iwe/) · [Discussions](https://github.com/iwe-org/iwe/discussions)

**在 IWE 之上构建？** CLI 和 MCP 服务器是目前受支持的集成方式。[告诉我们](https://github.com/iwe-org/iwe/discussions/362)你依赖哪些功能，这样我们就知道哪些不能改坏。

## 许可证

[Apache License 2.0](LICENSE-APACHE)
