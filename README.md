# Angaria MCP

English | [Türkçe](README.tr.md)

Angaria is a Mac app for Turkish e-Arşiv invoicing. It replaces the web pages of GİB's e-Arşiv Portal (GİB is the Turkish tax authority): you create invoices, sign them with the SMS code GİB sends to your phone, and download them from one native app. While Angaria is open, it also runs an MCP server, so Claude, Cursor, Codex and other AI assistants can create, sign and download your invoices and manage your customers and services.

This repository holds `angaria-mcp`, a small open source program that connects MCP clients which only run local programs (Claude Desktop) to that server. It also holds the files MCP directories read. The MCP server itself lives inside the Angaria app and works only while the app is open. When Angaria is closed, nothing listens, and every request gets the error "Angaria’ya bağlanılamadı. Angaria’yı aç." ("Could not connect to Angaria. Open Angaria.").

Website: [angaria.app/en](https://angaria.app/en)

## Requirements

- A Mac with macOS 11 or later.
- The [Angaria app](https://angaria.app/en), installed and open, with a license or the 7-day free trial. Angaria is a paid app.
- A GİB e-Arşiv Portal account (user code and password), saved in Angaria.
- The phone registered with GİB. Signing an invoice needs the SMS code GİB sends to it.

The Angaria app is in Turkish. The tools answer in Turkish, and your assistant can translate.

## Setup

The fastest way: open Angaria, go to the MCP page, and use Kurulum. Its Kurulum section lists 22 clients, among them Claude Code, Claude Desktop, Codex, Cursor, Gemini CLI, VS Code, Windsurf and Zed, and adds Angaria to each one with one click (Ekle).

By hand, every client gets the same address: `http://127.0.0.1:41234/mcp` (Streamable HTTP, no key).

### Claude Code

```bash
claude mcp add --transport http --scope user angaria http://127.0.0.1:41234/mcp
```

### Cursor

In `~/.cursor/mcp.json`:

```json
{"mcpServers":{"angaria":{"url":"http://127.0.0.1:41234/mcp"}}}
```

### Codex

In `~/.codex/config.toml`:

```toml
[mcp_servers.angaria]
url = "http://127.0.0.1:41234/mcp"
```

### Other clients

A client that takes Streamable HTTP servers by type and URL gets this entry, named `angaria`:

```json
{"type":"http","url":"http://127.0.0.1:41234/mcp"}
```

### Claude Desktop

Claude Desktop runs only local programs (stdio servers), so it needs `angaria-mcp`. The Angaria app already contains it. In `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "angaria": {
      "command": "/Applications/Angaria.app/Contents/MacOS/angaria-mcp"
    }
  }
}
```

Keep Angaria in your Applications folder. If you open it from Downloads or from the disk image, macOS runs it from a temporary location, and that path stops working.

You can also [build `angaria-mcp` from this repository](#build-from-source) and use the path of your build instead. When a release is published, its `angaria-mcp.mcpb` file installs the same program in Claude Desktop with a double click.

## Tools

The tools in the order the server lists them. A tool marked "Yes" works only when "Ajanlar onaylayabilir ve silebilir" ("Agents can sign and delete") is on in Angaria's MCP page. That switch is off by default. While it is off, these tools stay in the list and answer with an error that names the switch, so your assistant can tell you what to turn on.

| Tool | What it does | Needs the switch |
| --- | --- | --- |
| `list_invoices` | Lists invoices between two dates (31 days at most), filtered by status and recipient. |  |
| `get_invoice` | Gets an invoice's recipient, lines and totals, or GİB's own view of the invoice. |  |
| `create_invoice` | Creates an unsigned draft invoice with the same rules as the app's New invoice form. |  |
| `duplicate_invoice` | Creates a new draft from an existing invoice, dated today. |  |
| `download_invoice` | Saves the Angaria PDF, the e-Arşiv PDF or GİB's ZIP file to your Downloads folder. |  |
| `request_signing_sms` | Asks GİB to send an SMS code to your phone so drafts can be signed. | Yes |
| `sign_invoices` | Signs drafts with the SMS code you give the agent. A signed invoice is final. | Yes |
| `delete_draft` | Deletes an unsigned draft from GİB. Never touches signed invoices. | Yes |
| `list_customers` | Lists saved customers and their invoice defaults. |  |
| `get_customer` | Gets a customer's details and invoice defaults. |  |
| `create_customer` | Saves a new customer. |  |
| `update_customer` | Changes a customer's details or invoice defaults. | Yes |
| `delete_customer` | Deletes a saved customer. Invoices you already issued do not change. | Yes |
| `list_services` | Lists saved services with their prices, currencies and VAT rates. |  |
| `get_service` | Gets a service's price, unit and VAT rate. |  |
| `create_service` | Saves a new service. |  |
| `update_service` | Changes a service's name, price or VAT rate. | Yes |
| `delete_service` | Deletes a saved service. Invoices you already issued do not change. | Yes |
| `list_companies` | Lists the companies in Angaria and which one is selected. |  |
| `get_company_details` | Gets the company and bank details printed on the PDF, and the PDF design. |  |
| `update_company_details` | Changes the company and bank details printed on the PDF, including the IBAN. | Yes |
| `rename_company` | Changes the company name shown in Angaria's company list. | Yes |
| `set_company_logo` | Replaces the PDF logo with an image file on your Mac, or removes it. | Yes |
| `get_exchange_rate` | Gets the Turkish central bank (TCMB) buying rate for an invoice date. |  |
| `check_gib_connection` | Logs in to GİB with the saved user code, then logs out. The password never goes to the agent. |  |
| `lookup_taxpayer` | Gets the registered name and tax office for a Turkish tax ID (VKN or TCKN) from GİB. |  |
| `close_gib_session` | Closes a session that is still open on the e-Arşiv Portal. | Yes |
| `scan_gib_import` | Finds customers and services to save from your signed invoices. |  |
| `apply_gib_import` | Saves the customers and services you chose from the scan. Never changes saved ones. |  |
| `get_app_settings` | Gets the theme, the color palette and Angaria's version. |  |
| `update_app_settings` | Changes the theme or the color palette. |  |

## Example

> Invoice Apple for $5,000, service export

The assistant finds Apple in your saved customers (or adds it), then creates a draft in US dollars as an İstisna invoice (exempt from VAT, as services sold abroad are), with the TCMB buying rate for the invoice date. You see the draft in Angaria. To sign it from the chat, turn on the switch, ask the assistant to sign, and give it the SMS code GİB sends to your phone.

## Safety

- The server listens only on this Mac (`127.0.0.1`). It refuses a request whose `Host` is not a local address on its port, and any request with an `Origin` header, so web pages cannot reach it.
- There is no key. While Angaria is open, any program on this Mac can reach the server. When Angaria is closed, nothing listens.
- Signing always needs the SMS code GİB sends to your phone. The assistant cannot sign without the code you give it.
- Signing, deleting and changing company details (the IBAN on every PDF) work only when "Ajanlar onaylayabilir ve silebilir" is on. It is off by default.
- Your GİB password stays in the macOS Keychain. No tool reads or writes it.
- Angaria has no account and no cloud storage. Your invoices, customers and services stay on your Mac. The app talks only to GİB, TCMB (for exchange rates), Polar (for the license) and Angaria's update server.
- Angaria's MCP page lists every tool call in an activity log, with SMS codes and passwords masked.

## Build from source

You need macOS and [Rust](https://rustup.rs). `rust-toolchain.toml` installs both Mac targets.

```bash
cargo test
cargo build --release
```

The program is at `target/release/angaria-mcp`. To build the universal binary for Apple silicon and Intel and pack `dist/angaria-mcp.mcpb` (packing needs Node with `npx`):

```bash
./scripts/build.sh
```

`SKIP_PACK=1 ./scripts/build.sh` builds only the universal binary, at `dist/angaria-mcp`.

`angaria-mcp` reads JSON-RPC messages from standard input, one per line, sends each to `http://127.0.0.1:41234/mcp`, and writes each answer to standard output. It reads no file and never opens Angaria.

## License

The code in this repository is under the [MIT License](LICENSE). The Angaria app itself is proprietary and paid; its source is not public.
