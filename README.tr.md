# Angaria MCP

[English](README.md) | Türkçe

Angaria, e-Arşiv fatura kesmek için bir Mac uygulaması. GİB’in e-Arşiv Portalı’nın web sayfalarının yerini alır: faturayı oluşturur, GİB’in telefonuna gönderdiği SMS şifresiyle onaylar ve indirirsin, hepsi tek bir uygulamada. Angaria açıkken bir MCP sunucusu da çalıştırır. Böylece Claude, Cursor, Codex ve diğer yapay zekâ asistanları faturalarını oluşturabilir, onaylayabilir, indirebilir, müşterilerini ve hizmetlerini yönetebilir.

Bu depoda `angaria-mcp` var: yalnızca yerel program çalıştıran MCP istemcilerini (Claude Desktop) bu sunucuya bağlayan küçük, açık kaynak bir program. MCP dizinlerinin okuduğu dosyalar da burada. MCP sunucusunun kendisi Angaria uygulamasının içinde ve yalnızca uygulama açıkken çalışır. Angaria kapalıyken hiçbir şey dinlemez ve her istek "Angaria’ya bağlanılamadı. Angaria’yı aç." hatasını alır.

Web sitesi: [angaria.app](https://angaria.app)

## Gerekenler

- macOS 11 ya da sonrası yüklü bir Mac.
- [Angaria uygulaması](https://angaria.app), yüklü ve açık, lisanslı ya da 7 günlük ücretsiz denemede. Angaria ücretli bir uygulama.
- Angaria’ya kayıtlı bir GİB e-Arşiv Portalı hesabı (kullanıcı kodu ve şifre).
- GİB’e kayıtlı telefon. Fatura onaylamak, GİB’in bu telefona gönderdiği SMS şifresini ister.

## Kurulum

En hızlı yol: Angaria’yı aç, MCP sayfasına git ve Kurulum’u kullan. Kurulum bölümü 22 istemciyi listeler (aralarında Claude Code, Claude Desktop, Codex, Cursor, Gemini CLI, VS Code, Windsurf ve Zed var) ve Angaria’yı her birine tek tıkla (Ekle) ekler.

Elle eklerken her istemci aynı adresi alır: `http://127.0.0.1:41234/mcp` (Streamable HTTP, anahtar yok).

### Claude Code

```bash
claude mcp add --transport http --scope user angaria http://127.0.0.1:41234/mcp
```

### Cursor

`~/.cursor/mcp.json` dosyasına:

```json
{"mcpServers":{"angaria":{"url":"http://127.0.0.1:41234/mcp"}}}
```

### Codex

`~/.codex/config.toml` dosyasına:

```toml
[mcp_servers.angaria]
url = "http://127.0.0.1:41234/mcp"
```

### Diğer istemciler

Streamable HTTP sunucularını tür ve adresle alan bir istemciye, `angaria` adıyla şu kayıt girer:

```json
{"type":"http","url":"http://127.0.0.1:41234/mcp"}
```

### Claude Desktop

Claude Desktop yalnızca yerel program (stdio sunucusu) çalıştırır, bu yüzden `angaria-mcp` gerekir. Angaria uygulamasının içinde zaten var. `~/Library/Application Support/Claude/claude_desktop_config.json` dosyasına:

```json
{
  "mcpServers": {
    "angaria": {
      "command": "/Applications/Angaria.app/Contents/MacOS/angaria-mcp"
    }
  }
}
```

Angaria’yı Uygulamalar klasöründe tut. İndirilenler’den ya da disk görüntüsünden açarsan macOS onu geçici bir yerden çalıştırır ve bu yol çalışmaz.

İstersen `angaria-mcp`’yi [bu depodan derleyip](#kaynaktan-derleme) kendi derlemenin yolunu da kullanabilirsin. Bir sürüm yayınlandığında, içindeki `angaria-mcp.mcpb` dosyası aynı programı Claude Desktop’a çift tıkla kurar.

## Araçlar

Araçlar, sunucunun listelediği sırayla. "Evet" yazan araçlar yalnızca Angaria’nın MCP sayfasında "Ajanlar onaylayabilir ve silebilir" açıkken çalışır. Bu ayar varsayılan olarak kapalı. Kapalıyken bu araçlar listede kalır ve ayarın adını veren bir hatayla yanıt verir, böylece asistanın neyi açman gerektiğini sana söyleyebilir.

| Araç | Ne yapar | Ayar gerekir |
| --- | --- | --- |
| `list_invoices` | İki tarih arasındaki faturalar, en fazla 31 gün. Duruma ve alıcıya göre süzer. |  |
| `get_invoice` | Bir faturanın alıcısı, satırları ve tutarları ya da GİB görünümü. |  |
| `create_invoice` | İmzasız taslak oluşturur, Yeni fatura’daki kurallarla. |  |
| `duplicate_invoice` | Bir faturadan bugünün tarihiyle yeni bir taslak oluşturur. |  |
| `download_invoice` | Angaria PDF’ini, e-Arşiv PDF’ini ya da GİB’in ZIP’ini İndirilenler’e kaydeder. |  |
| `request_signing_sms` | Taslakları onaylamak için GİB’den telefonuna SMS şifresi ister. | Evet |
| `sign_invoices` | Ajana yazdığın SMS şifresiyle taslakları onaylar. Onaylanan fatura kesinleşir. | Evet |
| `delete_draft` | İmzasız bir taslağı GİB’den siler. Onaylı faturalara dokunmaz. | Evet |
| `list_customers` | Kayıtlı müşteriler ve fatura varsayılanları. |  |
| `get_customer` | Bir müşterinin bilgileri ve fatura varsayılanları. |  |
| `create_customer` | Yeni bir müşteri kaydeder. |  |
| `update_customer` | Bir müşterinin bilgilerini ya da varsayılanlarını değiştirir. | Evet |
| `delete_customer` | Bir müşteriyi kayıtlardan siler. Kestiğin faturalar etkilenmez. | Evet |
| `list_services` | Kayıtlı hizmetler, fiyatları, para birimleri ve KDV oranları. |  |
| `get_service` | Bir hizmetin fiyatı, birimi ve KDV oranı. |  |
| `create_service` | Yeni bir hizmet kaydeder. |  |
| `update_service` | Bir hizmetin adını, fiyatını ya da KDV oranını değiştirir. | Evet |
| `delete_service` | Bir hizmeti kayıtlardan siler. Kestiğin faturalar etkilenmez. | Evet |
| `list_companies` | Angaria’daki şirketler ve seçili olan. Araçlar seçili şirketle çalışır. |  |
| `get_company_details` | PDF’e basılan firma ve banka bilgileri, PDF tasarımı. |  |
| `update_company_details` | PDF’e basılan firma ve banka bilgilerini, IBAN dahil, değiştirir. | Evet |
| `rename_company` | Angaria’daki şirket listesinde görünen adı değiştirir. | Evet |
| `set_company_logo` | PDF’e basılan logoyu Mac’indeki bir dosyayla değiştirir ya da kaldırır. | Evet |
| `get_exchange_rate` | TCMB’nin bir fatura tarihi için döviz alış kuru. |  |
| `check_gib_connection` | Kayıtlı kullanıcı koduyla GİB’e girip çıkar. Şifre ajana gitmez. |  |
| `lookup_taxpayer` | Bir VKN ya da TCKN için GİB kaydındaki unvanı ya da adı soyadı ve vergi dairesini getirir. |  |
| `close_gib_session` | e-Arşiv portalında açık kalan oturumu kapatır. | Evet |
| `scan_gib_import` | Onaylı faturalardan kaydedilecek müşterileri ve hizmetleri bulur. |  |
| `apply_gib_import` | Taramada seçilen müşterileri ve hizmetleri kaydeder. Kayıtlı olanlara dokunmaz. |  |
| `get_app_settings` | Tema, renk paleti ve Angaria’nın sürümü. |  |
| `update_app_settings` | Temayı ya da renk paletini değiştirir. |  |

## Örnek

> Apple’a 5.000 dolarlık hizmet ihracatı faturası kes

Asistan Apple’ı kayıtlı müşterilerinde bulur (yoksa ekler), sonra fatura tarihinin TCMB döviz alış kuruyla, ABD doları cinsinden bir İstisna faturası taslağı oluşturur (yurt dışına satılan hizmet gibi, KDV’siz). Taslağı Angaria’da görürsün. Sohbetten onaylamak için ayarı aç, asistandan onaylamasını iste ve GİB’in telefonuna gönderdiği SMS şifresini ona yaz.

## Güvenlik

- Sunucu yalnızca bu Mac’te dinler (`127.0.0.1`). `Host` başlığı kendi portunda yerel bir adres olmayan isteği ve `Origin` başlığı olan her isteği reddeder, böylece web sayfaları ona ulaşamaz.
- Anahtar yok. Angaria açıkken bu Mac’teki her program sunucuya ulaşabilir. Angaria kapalıyken hiçbir şey dinlemez.
- Onay her zaman GİB’in telefonuna gönderdiği SMS şifresini ister. Asistan, senin verdiğin şifre olmadan onaylayamaz.
- Onaylamak, silmek ve firma bilgilerini (her PDF’teki IBAN) değiştirmek yalnızca "Ajanlar onaylayabilir ve silebilir" açıkken çalışır. Varsayılan olarak kapalı.
- GİB şifren macOS Anahtar Zinciri’nde kalır. Hiçbir araç onu okumaz ya da yazmaz.
- Angaria’nın hesabı ya da bulutu yok. Faturaların, müşterilerin ve hizmetlerin Mac’inde kalır. Uygulama yalnızca GİB, TCMB (döviz kurları için), Polar (lisans için) ve Angaria’nın güncelleme sunucusuyla konuşur.
- Angaria’nın MCP sayfası her araç çağrısını bir etkinlik kaydında gösterir, SMS şifreleri ve parolalar gizlenir.

## Kaynaktan derleme

macOS ve [Rust](https://rustup.rs) gerekir. `rust-toolchain.toml` iki Mac hedefini de kurar.

```bash
cargo test
cargo build --release
```

Program `target/release/angaria-mcp` konumunda olur. Apple silicon ve Intel için tek (universal) bir dosya derleyip `dist/angaria-mcp.mcpb` paketini oluşturmak için (paketleme `npx` ile Node ister):

```bash
./scripts/build.sh
```

`SKIP_PACK=1 ./scripts/build.sh` yalnızca universal dosyayı `dist/angaria-mcp` konumuna derler.

`angaria-mcp` standart girdiden her satırda bir JSON-RPC mesajı okur, her birini `http://127.0.0.1:41234/mcp` adresine gönderir ve her yanıtı standart çıktıya yazar. Hiçbir dosya okumaz ve Angaria’yı hiç açmaz.

## Lisans

Bu depodaki kod [MIT Lisansı](LICENSE) ile sunulur. Angaria uygulamasının kendisi tescilli ve ücretli, kaynak kodu açık değil.
