# IWE: 당신과 AI 에이전트가 함께 쓰는 Markdown 지식 베이스

[English](README.md) · [简体中文](README.zh-CN.md) · **한국어**

> 당신의 노트가 곧 에이전트의 메모리입니다. IWE는 그 노트를 함께 다듬는 곳입니다.

**[iwe.md](https://iwe.md/?ref=readme-header)** · **[Mac용 다운로드](https://iwe.md/app/?ref=readme-header)** · [문서](https://iwe.md/docs/?ref=readme-header) · [Reddit](https://www.reddit.com/r/iwe/)

[![Crates.io](https://img.shields.io/crates/v/iwe.svg)](https://crates.io/crates/iwe)
[![License](https://img.shields.io/crates/l/iwe.svg)](https://github.com/iwe-org/iwe/blob/master/LICENSE-APACHE)
[![Documentation](https://img.shields.io/badge/docs-iwe.md-blue)](https://iwe.md)

AI와의 대화는 너무 자주 처음부터 다시 시작됩니다. 프로젝트와 결정, 그 이유를 설명하고 나면 다음 세션에서 또다시 전부 설명하게 됩니다. 정작 당신의 노트는 에이전트가 닿지 않는 다른 곳에 있습니다.

더 나쁜 경우, 에이전트가 자체 내부 메모리에 무언가를 "기억"하지만, 무엇을 저장했는지 쉽게 확인할 수도 없고 잘못 기억한 내용을 고칠 수도 없습니다. 게다가 그 메모리는 당신이 함께 일하는 다른 에이전트로 함께 옮겨 가지 않습니다.

IWE는 당신과 AI 에이전트에게 **프로젝트 안에 바로 있는 하나의 공유 작업 공간**을 제공합니다. 서로 연결된 Markdown 문서들이 담긴 폴더로, 스펙, 계획, 결정 등이 들어갑니다. 당신의 아이디어를 그곳에 두고, 함께 알게 된 것을 에이전트에게 저장하게 하고, 작업이 커져 갈 때마다 그 맥락으로 돌아오세요. 이 문서들은 직접 읽고, 편집하고, 정리할 수 있으며 언제나 당신의 것입니다.

## 작업 방식 선택하기

IWE를 사용하는 방법은 네 가지입니다. 하루 일과에 맞게 자유롭게 조합하세요:

| 도구 | 사용 방법 |
| --- | --- |
| **[iWe for Mac](https://iwe.md/?mode=app&ref=readme-table)** | 네이티브 앱에서 글을 쓰고 탐색하며, 기존 Claude/Codex 에이전트를 연결합니다. |
| **[사용 중인 편집기](https://iwe.md/?mode=editor&ref=readme-table)** | Neovim, Helix, VS Code, Zed를 그대로 쓰면서 노트 탐색, 백링크, 자동 완성을 사용합니다. |
| **[CLI](https://iwe.md/docs/cli/?ref=readme-table)** | 터미널이나 스크립트에서 노트를 검색하고 관리합니다. |
| **[Claude/Codex](https://iwe.md/docs/agentic/?ref=readme-table)** | 대화를 통해 일하는 동안 에이전트가 메모리를 읽고, 쓰고, 정리하게 합니다. |

네 가지 모두 같은 Markdown 문서를 다룹니다. 편집기에서 스펙 초안을 쓰고, Claude/Codex와 함께 발전시키고, iWe 앱에서 둘러보거나 CLI로 검색하세요. 원하는 대로 조합할 수 있으며, 각 도구는 독립적으로도 작동합니다.

**AI는 직접 가져오세요.** IWE CLI와 iWe Mac 앱에는 AI 모델이 포함되어 있지 않습니다. 둘 다 그 자체로 노트를 위한 도구로 작동합니다. AI의 도움이 필요할 때는 기존 Claude/Codex 에이전트를 사용하세요. 터미널에서든 Mac 앱 안에서든 가능합니다.

<a href="https://iwe.md/?ref=readme-shot"><img src="https://iwe.md/images/app/GH-screen-1.1.png" alt="프로젝트 계획과 연결된 문서를 보여주는 iWe for Mac" width="820"></a>

<p>
  <a href="https://iwe.md/?ref=readme-shot"><img src="https://iwe.md/images/app/GH-screen-2.png" alt="차트를 활용한 버그 추적" width="32%"></a>
  <a href="https://iwe.md/?ref=readme-shot"><img src="https://iwe.md/images/app/GH-screen-3.png" alt="코드가 포함된 기술 문서" width="32%"></a>
  <a href="https://iwe.md/?ref=readme-shot"><img src="https://iwe.md/images/app/GH-screen-4.png" alt="릴리스 문서" width="32%"></a>
</p>

*계획 · 차트 · 기술 문서 · 릴리스 — 스크린샷을 클릭하면 iwe.md에서 iWe 앱이 실제로 작동하는 모습을 볼 수 있습니다.*

## 왜 IWE인가

- **공유되는 맥락.** 에이전트가 당신이 스스로를 위해 쓴 노트를 그대로 활용할 수 있습니다. 노트는 서로 연결되고, 하나의 노트가 여러 주제에 속할 수 있어서 아이디어가 이어지는 곳 어디로든 따라갈 수 있습니다.
- **읽을 수 있는 메모리.** 에이전트가 저장한 결정과 교훈은 평범한 노트이므로 직접 둘러보고, 고치고, 삭제할 수 있습니다.
- **노트는 언제나 당신의 것.** 모든 것은 당신의 컴퓨터 속 폴더에 있는 Markdown입니다. 백업하거나, 동기화하거나, git으로 관리하세요. 노트를 보관하는 데 IWE 클라우드 계정은 필요 없습니다. 다만 AI 에이전트를 연결할 때는 해당 에이전트의 설정과 계정 요구 사항을 따릅니다.

**[iwe.md에서 실제로 확인하기 →](https://iwe.md/?ref=readme-why)**

## iWe for Mac

노트를 위한 전용 공간을 원한다면 네이티브 Mac 앱을 사용하세요. 노트 폴더를 열면 빠르고 키보드 친화적인 작성·탐색 공간이 생깁니다. 기존 Claude/Codex 에이전트를 연결해 함께 작업할 수도 있습니다.

- **Claude/Codex 에이전트와 함께.** 이미 사용 중인 에이전트를 연결해 노트에 대해 질문하고, 초안을 쓰거나 재구성하게 하세요. 에이전트가 작업하는 동안 문서에 수정 사항이 나타나는 것을 볼 수 있습니다.
- **마음껏 실험하기.** 에이전트의 실행 한 번은 실행 취소 한 단계입니다. 아이디어를 시도해 보고, 마음에 들지 않으면 되돌리세요.
- **아이디어를 따라가기.** 관련 노트 사이를 오가고, 몇 번의 키 입력으로 검색하고, 문서 안에서 바로 Mermaid 다이어그램을 볼 수 있습니다. 편집기와 다른 도구들도 같은 파일로 계속 작업할 수 있습니다.

**[iwe.md에서 iWe for Mac 보기 →](https://iwe.md/?ref=readme-download)**

노트 폴더를 열고 지금 하고 있는 일에 대한 노트부터 시작해 보세요. Claude/Codex를 연결한 뒤에는 그 노트를 요약하거나 아이디어를 발전시키는 데 도움을 달라고 요청해 보세요.

## 사용 중인 편집기

[Neovim](https://iwe.md/docs/editors/neovim/), [Helix](https://iwe.md/docs/editors/helix/), [VS Code](https://iwe.md/docs/editors/vscode/), [Zed](https://iwe.md/docs/editors/zed/)에서 계속 작성하세요. IWE는 노트 전반에 링크 자동 완성, 백링크, 검색, 안전한 이름 변경 기능을 더해 줍니다.

사용하는 편집기의 가이드를 따라 노트 폴더를 열고, 자동 완성으로 두 노트를 연결해 보세요. AI 에이전트를 연결하지 않아도 IWE로 글을 쓸 수 있습니다.

## CLI

터미널이 더 편한가요? IWE로 노트를 검색하고, 만들고, 재구성하거나 스크립트에 포함하세요. CLI에는 AI가 내장되어 있지 않으며, 이 명령들은 독립적으로 작동합니다. 기존 Claude/Codex 에이전트도 IWE의 도구를 사용할 수 있습니다.

Homebrew로 설치하거나 [설치 가이드](https://iwe.md/docs/getting-started/installation/)에서 다른 방법을 선택하세요:

```bash
brew install iwe-org/iwe/iwe
```

그런 다음 노트 폴더 안에서 다음 명령을 실행하세요:

```bash
iwe init
iwe find
iwe find --lexical "project decisions"
```

`iwe find`는 노트 목록을 보여 주고, `--lexical`은 노트 내용을 검색합니다. 더 자세한 내용은 [CLI 레퍼런스](https://iwe.md/docs/cli/)와 [워크플로 예제](docs/cli-workflows.md)를 참고하세요.

## Claude/Codex

IWE는 에이전트만으로도 사용할 수 있습니다. Claude/Codex에게 결정을 저장하고, 지난 작업을 찾아보고, 배운 것을 정리하라고 요청하세요. 에이전트는 IWE를 통해 노트를 다루며, 당신은 언제든 그 파일을 직접 열어 볼 수 있습니다.

IWE는 특정 AI에 묶여 있지 않습니다. 같은 노트를 Claude Code, Claude Desktop, Codex, Gemini, Cursor, 그리고 [Model Context Protocol](https://modelcontextprotocol.io)을 지원하는 모든 도구에서 사용할 수 있습니다.

**Claude/Codex에게 메모리를 주세요** — Claude Code에서는 플러그인을 설치한 뒤, 기억하게 하고 싶은 프로젝트에서 `/iwe:init`을 실행하세요. Codex의 경우 아래의 에이전트 연결 단계를 따르세요.

```text
/plugin marketplace add iwe-org/skills
/plugin install iwe@iwe-org
/iwe:init
```

그런 다음 지금 하고 있는 작업의 결정 하나를 저장해 달라고 요청하고, 프로젝트에 생성된 노트를 열어 보세요.

**다른 에이전트 연결하기** — Homebrew가 설치되어 있다면 IWE를 설치하세요:

```bash
brew install iwe-org/iwe/iwe
```

노트 폴더에서 `iwe init`을 실행한 다음, **[에이전트 연결 가이드](https://iwe.md/docs/agentic/)**를 따라 해당 폴더를 작업 디렉터리로 하는 `iwec` MCP 서버를 추가하세요. 다른 설치 방법은 [설치 가이드](https://iwe.md/docs/getting-started/installation/)를 참고하세요.

연결이 끝나면 에이전트에게 물어보세요: "내 IWE 노트를 살펴보고 어떤 내용이 있는지 요약해 줘."

또는 에이전트가 직접 설정하게 할 수도 있습니다. 명령을 실행할 수 있는 에이전트에게 다음을 붙여 넣으세요:

```text
내 노트를 위해 IWE를 설정해 줘: 설치하고(brew install iwe-org/iwe/iwe,
npm install -g @iwe-org/iwe, 또는 cargo install iwe iwes iwec), 내 노트
디렉터리에서 `iwe init`을 실행한 다음, 작업 디렉터리를 그 폴더로 지정해
`iwec` MCP 서버를 추가해 줘.
문서: https://iwe.md/docs/agentic/
```

## 더 알아보기

- [시작하기](https://iwe.md/docs/getting-started/installation/) — 설치 및 설정
- [AI와 함께 일하기](https://iwe.md/docs/agentic/) — 에이전트 연결하기
- [사용 가이드](https://iwe.md/docs/getting-started/usage/) — 일상적인 워크플로
- [예제](https://iwe.md/docs/examples/) — 사람들이 IWE를 사용하는 방법
- [CLI 레퍼런스](https://iwe.md/docs/cli/)와 [MCP 서버](https://iwe.md/docs/agentic/mcp/) — 기술적인 내용이 궁금한 분들을 위해

**바로 쓸 수 있는 워크스페이스:** [dev-workspace](https://github.com/iwe-org/dev-workspace) — 코딩 에이전트를 위한 프로젝트 메모리 · [marketing-workspace](https://github.com/iwe-org/marketing-workspace) — 마케팅 에이전트를 위한 캠페인 메모리.

## 참여하기

직접 노트로 IWE를 써 보고 있나요? 무엇이 유용했고 어디서 막혔는지 알려 주세요. 질문, 아쉬운 점, 사용 사례 모두 [Discussions](https://github.com/iwe-org/iwe/discussions)에서 환영합니다.

IWE는 오픈 소스입니다. [버그](https://github.com/iwe-org/iwe/issues)를 제보하거나 [문서](docs/) 개선을 도와주실 수도 있습니다.

**커뮤니티:** [Twitter/X](https://x.com/iwe_md) · [Reddit](https://www.reddit.com/r/iwe/) · [Discussions](https://github.com/iwe-org/iwe/discussions)

**IWE를 기반으로 개발하고 있나요?** 현재 지원되는 통합 방식은 CLI와 MCP 서버입니다. 무엇에 의존하고 있는지 [알려 주시면](https://github.com/iwe-org/iwe/discussions/362) 무엇을 깨뜨리지 말아야 할지 알 수 있습니다.

## 라이선스

[Apache License 2.0](LICENSE-APACHE)
