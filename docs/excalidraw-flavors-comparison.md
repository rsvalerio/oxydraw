# Excalidraw Flavors — Feature Comparison

A side-by-side comparison of the Excalidraw ecosystem, organized so you can see exactly where **OxyDraw** can differentiate.

**Legend:** ✅ = yes · ➖ = partial / limited · ❌ = no · ❓ = unknown / your call

## The flavors being compared

| Flavor | What it is | License / model |
|---|---|---|
| **Editor (npm)** | `@excalidraw/excalidraw` — the React drawing component everything else embeds | MIT, free |
| **excalidraw.com** | Official free hosted web app (a showcase of the editor) | Free |
| **Excalidraw+** | Official paid product: cloud, teams, enterprise | ~$6–7/user/mo |
| **Self-hosted** | The open-source app run on your own server (e.g. Docker) | MIT, infra cost only |
| **Obsidian plugin** | The editor embedded inside Obsidian for visual note-taking (PKM) | Free, community |
| **Excalimate** | The editor + an animation/keyframe layer on top | Commercial |
| **OxyDraw** | **Your flavor — to be defined** | ❓ |

> Note on scope: Tldraw, Penpot, draw.io, Miro, and FigJam are *competitors*, not flavors — they don't embed the Excalidraw editor, so they're left out of these tables and summarized at the end as competitive context.

---

## 1. Core drawing & canvas

| Feature | Editor | excalidraw.com | Excalidraw+ | Self-hosted | Obsidian | Excalimate | OxyDraw |
|---|---|---|---|---|---|---|---|
| Infinite canvas | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❓ |
| Hand-drawn style | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❓ |
| Core shapes (rect, ellipse, arrow, text, freedraw) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❓ |
| Image support | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❓ |
| Shape / stencil libraries | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❓ |
| Dark mode | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❓ |
| Custom color palettes | ➖ | ➖ | ➖ | ➖ | ✅ | ➖ | ❓ |
| LaTeX / math embeds | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❓ |
| Animation / keyframes | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❓ |

## 2. Collaboration & sharing

| Feature | Editor | excalidraw.com | Excalidraw+ | Self-hosted | Obsidian | Excalimate | OxyDraw |
|---|---|---|---|---|---|---|---|
| Real-time co-editing | ❌ (DIY) | ✅ | ✅ | ✅ | ❌ | ➖ | ❓ |
| End-to-end encryption | ❌ | ✅ | ✅ | ✅ | n/a | ❓ | ❓ |
| Shareable links | ❌ | ✅ | ✅ | ✅ | ❌ | ✅ | ❓ |
| Read-only / view links & embeds | ❌ | ➖ | ✅ | ➖ | ✅ | ✅ | ❓ |
| Access management (edit/view rights) | ❌ | ❌ | ✅ | ❌ | n/a | ➖ | ❓ |
| Comments | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❓ |
| Voice hangouts / screenshare | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❓ |
| Team / workspace management | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❓ |

## 3. Storage & organization

| Feature | Editor | excalidraw.com | Excalidraw+ | Self-hosted | Obsidian | Excalimate | OxyDraw |
|---|---|---|---|---|---|---|---|
| Local file save (`.excalidraw`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❓ |
| Cloud storage / sync | ❌ | ➖ (1 scene) | ✅ (unlimited) | ❓ (DIY) | ➖ (vault) | ✅ | ❓ |
| Folders / scene organization | ❌ | ❌ | ✅ | ❌ | ✅ | ➖ | ❓ |
| Dashboard | ❌ | ❌ | ✅ | ❌ | ✅ (file tree) | ➖ | ❓ |
| Version history | ❌ | ❌ | ❌ | ❌ | ➖ (git/sync) | ❌ | ❓ ← **gap** |

## 4. Presentation & output

| Feature | Editor | excalidraw.com | Excalidraw+ | Self-hosted | Obsidian | Excalimate | OxyDraw |
|---|---|---|---|---|---|---|---|
| Export PNG / SVG | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❓ |
| Export to clipboard | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❓ |
| Slide / frame presentations | ❌ | ❌ | ✅ | ❌ | ❌ | ➖ | ❓ |
| Live online presentation mode | ❌ | ❌ | ✅ | ❌ | ❌ | ✅ (playback) | ❓ |
| Animated export (MP4/GIF/Lottie) | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❓ |
| PWA / offline | n/a | ✅ | ✅ | ✅ | ✅ (native) | ❓ | ❓ |

## 5. AI features

| Feature | Editor | excalidraw.com | Excalidraw+ | Self-hosted | Obsidian | Excalimate | OxyDraw |
|---|---|---|---|---|---|---|---|
| Generative AI prompting | ❌ | ➖ (limited) | ✅ (extended) | ❌ | ➖ | ➖ | ❓ |
| Text/diagram → drawing | ❌ | ➖ | ➖ | ❌ | ➖ | ➖ | ❓ |
| Sketch → code/UI ("make real") | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❓ ← **gap** |
| AI via MCP / agent integration | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ (MCP server) | ❓ |

## 6. Developer & extensibility

| Feature | Editor | excalidraw.com | Excalidraw+ | Self-hosted | Obsidian | Excalimate | OxyDraw |
|---|---|---|---|---|---|---|---|
| Embeddable React component | ✅ | n/a | n/a | n/a | ✅ | ✅ | ❓ |
| Imperative API (refs) | ✅ | n/a | n/a | n/a | ✅ | ❓ | ❓ |
| TypeScript types | ✅ | n/a | n/a | n/a | ✅ | ❓ | ❓ |
| Plugin / script engine | ❌ | ❌ | ❌ | ❌ | ✅ (automation) | ❌ | ❓ |
| Open file format | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❓ |
| OCR | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❓ |

## 7. Hosting, deployment & pricing

| Aspect | Editor | excalidraw.com | Excalidraw+ | Self-hosted | Obsidian | Excalimate | OxyDraw |
|---|---|---|---|---|---|---|---|
| Where it runs | your app | Excalidraw's cloud | Excalidraw's cloud | your server | local device | Excalimate cloud | ❓ |
| Data control / privacy | full | low | low | full | full (local) | medium | ❓ |
| SSO / SAML | ❌ | ❌ | ➖ (enterprise) | ❌ | ❌ | ❌ | ❓ ← **gap** |
| SOC 2 / DPA / compliance | n/a | ➖ | ✅ | DIY | n/a | ❓ | ❓ |
| Deep integrations (Jira, Confluence, GitHub, Slack) | ❌ | ❌ | ❌ | ❌ | ➖ (Obsidian) | ❌ | ❓ ← **gap** |
| Pricing | free | free | ~$6–7/user/mo | infra only | free | paid | ❓ |

---

## Where OxyDraw can differentiate

The ecosystem is crowded on **core drawing** (every flavor has it) and **basic collaboration** (the official cloud owns it). The open white space — features almost nobody ships — is where a new flavor wins:

1. **Structured / smart diagramming.** Auto-layout, snapping to grids, templates, and shape intelligence (UML, flowcharts, BPMN). Excalidraw is deliberately "rough" and *not* a formal modeling tool — a flavor that keeps the hand-drawn charm but adds structure-on-demand fills a real gap teams hit as they scale.

2. **Version history & audit trail.** Effectively missing across all flavors. A first-class timeline/branching history would be a standout, especially for teams and regulated industries.

3. **Deep integrations.** None of the flavors embed natively into Jira, Confluence, GitHub, Slack, or Linear. This is the single most-requested "growth" feature for teams outgrowing Excalidraw.

4. **Privacy-first / EU-hosted / GDPR-native.** The official cloud is low on data control. A flavor that is self-host-friendly *and* offers an EU-hosted, GDPR-native managed tier (data residency, DPA, encryption by default) addresses a market the official product underserves. This pairs well with a low-price SaaS subscription model.

5. **Enterprise controls out of the box.** SSO/SAML, granular permissions, and admin controls are gated behind enterprise or absent. Shipping these affordably is a wedge.

6. **AI as a core capability, not an add-on.** Sketch → working code/UI ("make real"-style), AI-assisted cleanup of rough sketches, or agent/MCP-driven diagram generation. Excalidraw's AI is limited; Excalimate is the only flavor leaning into MCP. A flavor that makes AI the headline differs sharply from the pack.

7. **A specific vertical.** Every successful flavor narrows: Obsidian = visual note-taking (PKM), Excalimate = animation. OxyDraw picking one sharp audience (e.g. system-design interviews, infra/architecture diagramming with cost estimates, education, incident response) beats being "Excalidraw but slightly different."

### Suggested positioning sentence to fill in

> **OxyDraw is Excalidraw for `______`, adding `______` and `______` that no other flavor offers, hosted `______`.**

---

*Compiled June 2026. Feature availability for third-party flavors changes frequently; verify before committing to a build plan. Some cells marked ➖/❓ reflect features that exist informally (e.g. via sync or DIY) rather than as first-class product features.*
