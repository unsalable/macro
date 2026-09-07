# FlowMacro — Mimari Plan

> Onay bekleyen doküman. Onaylandıktan sonra Faz 1'den itibaren kodlama başlar.

---

## 0. Ortam Durumu (doğrulandı)

| Bileşen | Durum |
|---|---|
| Node.js | v24.16.0 ✅ |
| npm | 11.13.0 ✅ |
| Rust / Cargo | 1.98.1 ✅ *(bu oturumda kuruldu)* |
| MSVC Build Tools | Visual Studio 18 ✅ |
| WebView2 Runtime | 152.0.4191.66 ✅ |
| Proje klasörü | `C:\Users\melik\Desktop\ai\macro` — **boş**, sıfırdan kurulum |

Tauri 2 için gereken her şey hazır. Silinecek mevcut yapı yok (§80).

---

## 1. Teknoloji Stack'i

### Frontend

| Katman | Seçim | Gerekçe |
|---|---|---|
| Runtime | **Tauri 2.x** (WebView2) | ~8-15 MB RAM, hızlı açılış, native tray, gerçek .msi/.exe |
| UI | **React 19 + TypeScript (strict)** | §72 type safety |
| Build | **Vite 7** | HMR + hızlı prod build |
| Stil | **Tailwind CSS v4** | `@theme` ile CSS değişkeni tabanlı token sistemi |
| Bileşen tabanı | **shadcn/ui konvansiyonu** | **21st.dev bileşenleri birebir bunun üzerine oturuyor** |
| Animasyon | **motion** (Framer Motion v12, `motion/react`) | 21st.dev'in standart bağımlılığı; GPU dostu transform/opacity |
| İkon | **lucide-react** | §35 (emoji yok) |
| State | **Zustand** | §50 |
| Drag & Drop | **@dnd-kit/core + sortable** | §15 |
| Toast | **sonner** | §55 |
| i18n | **i18next + react-i18next** | EN + TR |
| Font | **Inter** (lokal woff2, bundle içinde) | §36 — network isteği yok (§74) |

### Backend (Rust)

| Katman | Seçim | Gerekçe |
|---|---|---|
| Input enjeksiyon | **`windows` crate → `SendInput`** | Mouse4/5, scroll, absolute/relative, scan-code desteği |
| Global yakalama | **`SetWindowsHookExW` (WH_KEYBOARD_LL / WH_MOUSE_LL)** | Event-driven → **idle'da ~%0 CPU** (§49) |
| Zamanlama | `winmm::timeBeginPeriod(1)` + hibrit sleep/spin | 100+ CPS için gerekli |
| Serileştirme | `serde` + `serde_json` | §51 |
| Eşzamanlılık | `std::thread` + `crossbeam-channel` + `parking_lot` | Macro worker (§48) |
| Rastgelelik | `fastrand` | §13 jitter |
| Tray | Tauri 2 yerleşik `tray-icon` | §43 |
| Autostart | `tauri-plugin-autostart` | §44 |
| Dosya seçici | `tauri-plugin-dialog` | §23 import/export |

**Kullanılmayacak:** `tauri-plugin-global-shortcut`. Gerekçe: yalnızca key-**down** olayı veriyor; §19'daki **Hold modu** için key-**up** de şart. Zaten low-level keyboard hook'umuz olacağı için hotkey'leri de oradan yönetmek hem tutarlı hem tek kaynak.

---

## 2. Kritik Teknik Kararlar

### 2.1 Input enjeksiyonu — neden scan code?

`SendInput` ile klavye olayı gönderirken **`KEYEVENTF_SCANCODE`** kullanılacak, sadece virtual-key değil.
Sebep: DirectInput / RawInput kullanan birçok oyun VK-only enjeksiyonu görmez, scan code'u görür. Text Input action'ı için ise `KEYEVENTF_UNICODE`.

Mouse tarafı:
- Mouse4/5 → `MOUSEEVENTF_XDOWN` / `XUP` + `mouseData = XBUTTON1 | XBUTTON2`
- Scroll → `MOUSEEVENTF_WHEEL`, `mouseData = ±120 × amount`
- Absolute move → koordinatlar sanal ekrana göre 0–65535 aralığına normalize edilir (çok monitörlü kurulumda doğru çalışması için)
- Relative move → `MOUSEEVENTF_MOVE` ham delta

### 2.2 Global hook — feedback loop koruması

Recorder ve hotkey dinleyicisi kendi gönderdiğimiz input'u tekrar yakalarsa sonsuz döngü olur.
Çözüm: hook callback'inde `KBDLLHOOKSTRUCT.flags & LLKHF_INJECTED` / `MSLLHOOKSTRUCT.flags & LLMHF_INJECTED` bayrağı kontrol edilip **kendi ürettiğimiz olaylar atlanır**.

Hook callback'i Windows tarafından **çok kısa sürede dönmek zorunda** (aksi halde OS hook'u devre dışı bırakır). Bu yüzden callback içinde iş yapılmaz: olay lock-free kanala push edilir, ayrı bir thread işler. Hook'u kuran thread'in kendi `GetMessageW` mesaj döngüsü olacak.

### 2.3 Zamanlama doğruluğu

Windows varsayılan timer çözünürlüğü ~15.6 ms → 10 CPS bile tutmaz.
- Macro çalışırken `timeBeginPeriod(1)`, durunca `timeEndPeriod(1)` (sürekli açık bırakmak sistem geneli güç tüketimini artırır)
- **Hibrit bekleme:** kalan süre > 2 ms ise `thread::sleep(kalan − 1.5 ms)`, sonra `spin_loop()` ile hassas bitiş
- **Drift önleme:** `deadline += interval` (mutlak zaman çizelgesi). `sleep(interval)` biriktirme yapılmaz — 10 dakikalık macro'da saniyelerce kayma olurdu

### 2.4 Threading modeli (§48)

```
UI thread (WebView)  ──tauri::command──►  Engine Handle (Arc)
                                              │
Input Hook Thread ──channel──►  Engine Worker Thread (dedicated OS thread)
(WH_*_LL msg loop)                            │
                                              ├─ AtomicU8 state: Idle/Running/Paused/Stopping
                                              ├─ Condvar (pause/resume)
                                              └─ throttled event ──► UI (max 10 Hz)
```

- STOP = atomic bayrak; spin döngüsünün her turunda okunur → **milisaniye altı iptal** (§61)
- Engine her tıklamada UI'a event yollamaz; durum/sayaç güncellemesi 10 Hz'e throttle edilir (§3: gereksiz re-render yok)

### 2.5 Emergency Stop (§61)

- Varsayılan hotkey'ler: **F6 Start / F7 Stop / F8 Pause** (§18)
- **Emergency Stop varsayılanı: `F12`** — kullanıcı değiştirebilir
- **Kaldırılamayan güvenlik ağı:** 500 ms içinde **3 kez `Esc`** → engine koşulsuz durur. Hotkey yanlış atansa bile kullanıcı kilitli kalmaz

### 2.6 Depolama (§51)

`%APPDATA%\FlowMacro\`

```
settings.json          uygulama ayarları + hotkeys
profiles.json          tüm profiller ve macro'ları
history.jsonl          append-only, son 500 kayıtla sınırlı
```

- **Atomik yazma:** temp dosyaya yaz → `rename` (yazma sırasında crash = veri kaybı yok)
- Her dosyada `schemaVersion` alanı; `#[serde(default)]` ile ileri uyumluluk
- Import'ta tam validasyon: bozuk JSON crash ettirmez, kullanıcı dostu hata döner (§23, §40, §41)
- SQLite değil: veri hacmi küçük, JSON hem taşınabilir hem export formatıyla aynı

### 2.7 Pencere (§34)

- `decorations: false` + custom title bar, `data-tauri-drag-region`
- `minWidth: 1000, minHeight: 650` (§33)
- Windows 11 yuvarlak köşe: `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND)`
- Close → ayara göre tray'e minimize (§45), gerçek çıkış tray menüsünden

---

## 3. Klasör Yapısı

```
macro/
├─ src/
│  ├─ components/
│  │  ├─ ui/                shadcn + 21st.dev tabanlı primitive'ler
│  │  │  ├─ button.tsx  card.tsx  switch.tsx  slider.tsx
│  │  │  ├─ dialog.tsx  select.tsx  tabs.tsx  tooltip.tsx
│  │  │  └─ sonner.tsx  number-ticker.tsx  hotkey-input.tsx
│  │  ├─ layout/            TitleBar, Sidebar, PageShell, PageTransition
│  │  ├─ macro/             ActionCard, ActionList(dnd), ActionPalette, LoopEditor
│  │  ├─ mouse/             MouseVisualizer(SVG), ButtonConfigPanel, CpsControl
│  │  ├─ keyboard/          KeyboardVisualizer, KeyCap, ComboBuilder
│  │  ├─ recorder/          RecordButton, Timeline, TimelineEvent
│  │  └─ common/            EmptyState, StatusPill, ConfirmDialog, Skeleton
│  ├─ pages/                Home, Macros, Editor, Recorder, Profiles, History, Settings
│  ├─ layouts/              AppLayout
│  ├─ hooks/                useEngineStatus, useHotkeyCapture, useUndoRedo, useShortcuts
│  ├─ stores/               engineStore, macroStore, profileStore, settingsStore, uiStore
│  ├─ services/             tauri.ts (command wrapper), validation.ts, importExport.ts
│  ├─ i18n/                 index.ts, en.json, tr.json
│  ├─ lib/                  motion.ts (token'lar), cn.ts, cps.ts, format.ts
│  ├─ types/                macro.ts, engine.ts, settings.ts   ← Rust ile birebir eş
│  └─ styles/               theme.css (tüm design token'lar)
│
└─ src-tauri/
   ├─ src/
   │  ├─ main.rs / lib.rs
   │  ├─ commands/          macro_cmds.rs, engine_cmds.rs, config_cmds.rs, recorder_cmds.rs
   │  ├─ input/
   │  │  ├─ inject.rs       SendInput sarmalayıcı
   │  │  ├─ hook.rs         WH_KEYBOARD_LL / WH_MOUSE_LL
   │  │  ├─ keycodes.rs     VK ↔ scan code ↔ isim eşlemesi
   │  │  └─ timing.rs       hibrit sleep + timeBeginPeriod guard
   │  ├─ macro/
   │  │  ├─ model.rs        Action, Macro, Loop (serde)
   │  │  ├─ engine.rs       worker thread + state machine
   │  │  └─ scheduler.rs    deadline hesabı + jitter
   │  ├─ hotkeys/           registry.rs, matcher.rs (toggle/hold)
   │  ├─ recorder/          capture.rs
   │  ├─ config/            settings.rs, profiles.rs
   │  ├─ storage/           atomic_json.rs, history.rs
   │  └─ tray.rs
   ├─ icons/
   ├─ Cargo.toml
   └─ tauri.conf.json
```

**Kural:** UI hiçbir zaman doğrudan low-level API'ye dokunmaz (§38); tek geçit `services/tauri.ts`.
Hiçbir dosya 300 satırı geçmeyecek; `App.tsx` sadece router + provider (§72).

---

## 4. Tasarım Sistemi

### Renk token'ları (§26, §27)

```css
/* Light — varsayılan */
--background:        #F8F6F1   /* sıcak beyaz */
--card:              #FFFFFF
--card-muted:        #FCFAF6
--border:            #E9E4DA
--border-strong:     #DCD5C8
--foreground:        #2B2622   /* koyu sıcak gri */
--muted-foreground:  #7C7368
--accent:            #8A7B68   /* muted warm brown — çok sınırlı kullanım */
--success:           #6E8B6A   /* muted green */
--danger:            #A8615A   /* terracotta — parlak kırmızı DEĞİL (§56) */
--ring:              rgba(138,123,104,.35)

/* Dark — saf siyah yok */
--background:        #171614
--card:              #1F1D1A
--border:            #2E2B26
--foreground:        #EDE8DF
--muted-foreground:  #9A9186
```

Token isimleri **shadcn konvansiyonuyla birebir aynı** (`--background`, `--foreground`, `--card`, `--border`, `--muted`, `--accent`, `--ring`). Sonucu: **21st.dev'den alınan her bileşen `bg-background` / `text-foreground` kullandığı için krem temayı otomatik devralıyor** — yeniden renklendirme işi neredeyse sıfır.

### Gölge / radius / spacing

```
shadow-soft:  0 1px 2px rgba(43,38,34,.04), 0 4px 12px rgba(43,38,34,.05)
shadow-lift:  0 2px 4px rgba(43,38,34,.05), 0 10px 24px rgba(43,38,34,.07)
radius:       sm 10px · md 14px · lg 18px
spacing:      8 / 12 / 16 / 24 / 32 / 48   (§37 — 8px sistemi)
```

### Motion token'ları (§67)

```ts
duration = { fast: .12, normal: .20, slow: .30 }
ease     = [0.22, 1, 0.36, 1]                                     // tek merkezi eğri
spring   = { type: 'spring', stiffness: 380, damping: 32, mass: .8 }  // bounce yok
stagger  = .035                                                   // kart giriş gecikmesi
```

Tek bir `<MotionConfig>` sağlayıcısı; **Settings → Performance → Animation Quality** (Low/Balanced/High) ve OS `prefers-reduced-motion` bu token'ları global olarak kısar (§46, §68). Hiçbir bileşen kendi kafasına göre eğri tanımlamaz.

---

## 5. 21st.dev Bileşen Haritası

Stack'i shadcn + Tailwind + motion seçmemin sebebi bu: 21st.dev bileşenleri **doğrudan** projeye düşüyor.

| Kaynak (21st.dev) | Nerede kullanılacak | Uyarlama |
|---|---|---|
| `@unlumen/sidebar-001` — *Animated Sidebar* (spring hover highlight + animasyonlu aktif gösterge çubuğu) | **Sidebar** (§5, §28) — aktif göstergenin teleport etmeyip kayması tam olarak bu bileşenin davranışı | Krem palet, docs-nav yerine 6 sayfa, collapse desteği |
| `@shadcn/sonner` | **Toast sistemi** (§55) | Sağ alt, soft shadow, slide + fade |
| `@preetsuthar17/draggable-list` + `@nikhiljainsam/draggable-priority-list` | **Macro Editor action sıralama** (§15) | dnd-kit ile birleştirilecek; drag'de hafif scale + placeholder |
| *Number Ticker* (Magic UI) | **Dashboard sayaçları**, canlı CPS göstergesi (§7) | Krem palet |
| shadcn/ui primitive'leri (Dialog, Switch, Select, Slider, Tabs, Tooltip, DropdownMenu) | Tüm uygulama tabanı | Radius / gölge / renk token'ları ile |
| Animated Tabs | **Settings kategorileri**, Mouse panel modları | Ortak motion token'ları |

**Yöntem:** her bileşenin kaynağı implementasyon anında 21st.dev'den tekrar çekilip (`Component.tsx` sekmesi) `src/components/ui/` altına indirilecek, sonra krem palete ve motion token'larına göre yeniden düzenlenecek. Lisanslar MIT.

**Özel yazılacaklar (21st.dev'de karşılığı yok):** Mouse Visualizer (SVG, 5 tıklanabilir bölge), Keyboard Visualizer, Recorder Timeline, hotkey capture input.

---

## 6. Veri Modeli

TypeScript tipleri ile Rust struct'ları **birebir eşleşecek** (`serde(rename_all = "camelCase")`).

```ts
type MacroAction =
  | { id: string; type: 'mouse_click'; button: MouseButton; mode: ClickMode;
      count: number; intervalMs: number; durationMs: number;
      delayBeforeMs: number; delayAfterMs: number }
  | { id: string; type: 'mouse_move'; mode: 'absolute' | 'relative';
      x: number; y: number; durationMs: number; curve: 'linear' | 'smooth' }
  | { id: string; type: 'mouse_scroll'; direction: 'up' | 'down'; amount: number }
  | { id: string; type: 'key'; action: 'press' | 'down' | 'up';
      code: string; modifiers: Modifier[]; durationMs: number }
  | { id: string; type: 'text'; value: string; perCharDelayMs: number }
  | { id: string; type: 'delay'; durationMs: number }
  | { id: string; type: 'repeat'; times: number; actions: MacroAction[] }

interface Macro {
  id: string; name: string; enabled: boolean;
  hotkey: Hotkey | null;
  activation: 'toggle' | 'hold';
  actions: MacroAction[];
  loop: { mode: 'none' | 'infinite' | 'count' | 'duration'; count?: number; durationMs?: number };
  randomization: { enabled: boolean; jitterMs: number };   // varsayılan false (§13)
  stats: { runCount: number; lastRunAt: string | null };
}

interface Profile { id: string; name: string; color: ProfileColor; macros: Macro[] }
```

`schemaVersion` her dosyada. **CPS ↔ interval çift yönlü bağlanır** (§11): `interval = 1000 / cps`, birini değiştirmek diğerini günceller; store seviyesinde tek gerçek kaynak `intervalMs`.

---

## 7. Tauri Command API Yüzeyi

```
engine_start(macroId)      engine_stop()        engine_pause()      engine_resume()
engine_status()            engine_test_action(action)
macro_save(macro)          macro_delete(id)     macro_list()
macro_export(id, path)     macro_import(path) -> Result<Macro, ValidationError>
profile_list / save / delete / switch(id)
settings_get / set         hotkeys_set(map)
recorder_start / stop -> RecordedEvent[]
history_list(limit)        history_clear()
```

**Event'ler (Rust → UI):** `engine:status`, `engine:progress` (10 Hz throttle), `recorder:event`, `hotkey:triggered`, `engine:error`.

---

## 8. Ekranlar

| # | Ekran | Öne çıkanlar |
|---|---|---|
| 1 | **Home / Dashboard** | Karşılama, Macro Engine kontrol kartı (Ready / Running / Paused, Start), son kullanılan macro kartları (§6, §7) |
| 2 | **Macros** | Liste/grid, arama, enable toggle, boş durum (§53) |
| 3 | **Macro Editor** | dnd-kit action listesi, action palette, loop editörü, Preview (§59), Test (§60), Undo/Redo (§58) |
| 4 | **Recorder** | Kayıt kontrolü, canlı `● Recording`, timeline (§16, §17) |
| 5 | **Mouse Config** | SVG mouse visualizer (5 bölge), sağ panel detay ayarları, CPS/interval (§10, §11, §12, §62–64) |
| 6 | **Keyboard Config** | Tam klavye visualizer, tuş seçimi, modifier + kombo (§14) |
| 7 | **Profiles** | Profil kartları, sınırlı renk paleti, animasyonlu geçiş (§22, §65) |
| 8 | **History** | Zaman damgalı event listesi (§24) |
| 9 | **Settings** | Sol kategori + sağ ayar listesi; General / Appearance / Hotkeys / Performance / Engine / Advanced (§25, §66) |
| — | **Tray** | Open · Start · Pause · Stop · Exit (§43) |
| — | **Onboarding** | Tek ekran welcome (§52) |

---

## 9. i18n

`react-i18next`, `en.json` + `tr.json`. Varsayılan: sistem diline göre otomatik, Settings → General'dan değiştirilebilir. Tüm string'ler baştan anahtarlı yazılacak (sonradan sökmek pahalı). Sayı/tarih formatı `Intl` ile.

---

## 10. Faz Planı

Her fazın sonunda **build gate** çalışır (§81): `cargo check` → `npx tsc --noEmit` → `npm run build`. Build kırıksa yeni özelliğe geçilmez.

| Faz | İçerik | Çıktı |
|---|---|---|
| **1** | Proje kurulumu: Tauri 2 + Vite + React + TS strict + Tailwind v4 + token'lar + i18n iskeleti | `npm run tauri dev` ile açılan pencere |
| **2** | Native input engine: `inject.rs`, `keycodes.rs`, `timing.rs`, `hook.rs` | Rust unit testleri; tüm buton/tuş tipleri gönderilebiliyor |
| **3** | Veri modeli + storage: model.rs, atomic_json, validation, TS tipleri | Kaydet / yükle / import / export çalışıyor |
| **4** | Macro execution engine: worker thread, state machine, loop, jitter, emergency stop | F6/F7/F8 ile gerçek macro çalışıyor |
| **5** | UI mimarisi: AppLayout, custom title bar, **21st.dev Sidebar**, sayfa geçişleri, motion sistemi | Gezinilebilir kabuk |
| **6** | Dashboard + Macros listesi + empty state + toast | §6, §7, §53, §55 |
| **7** | Macro Editor: dnd-kit, action palette, undo/redo, preview | §8, §15, §58, §59 |
| **8** | Mouse Visualizer + CPS/interval + click modes | §10–§12, §62–§64 |
| **9** | Keyboard Visualizer + kombo | §14 |
| **10** | Recorder + timeline | §16, §17 |
| **11** | Profiles | §22, §65 |
| **12** | Settings (6 kategori) + tema + animation quality | §25, §26, §46, §66 |
| **13** | System tray + autostart + minimize to tray | §43–§45 |
| **14** | Animasyon cilası + reduced motion + görsel kalite geçidi | §82 |
| **15** | Performans: idle CPU ölçümü, render profili, memory leak kontrolü | §3, §47, §49 |
| **16** | Test: Rust unit + timing doğruluk testi + manuel senaryolar | §81 |
| **17** | Paketleme: ikon, versiyon, NSIS `.exe` + `.msi` | §75 |

---

## 11. Riskler ve Karşılıkları

| Risk | Karşılık |
|---|---|
| Antivirüs, global hook + SendInput kullanan uygulamayı şüpheli görebilir | Beklenen davranış; imzasız build'de normal. README'de not; ileride code signing |
| Yüksek CPS'te (>200) Windows scheduler jitter'ı | Hibrit spin-wait + `timeBeginPeriod(1)`; ölçülen gerçek CPS UI'da gösterilecek |
| Bazı oyunlar RawInput ile enjekte input'u filtreler | Scan-code enjeksiyonu kullanılıyor; sınırlama README'de açıkça yazılacak |
| Yönetici olarak çalışan pencerelerde macro çalışmaz (UIPI) | Kullanıcı dostu hata mesajı + "yönetici olarak çalıştır" önerisi |
| Hook thread'i takılırsa Windows hook'u düşürür | Callback'te iş yapılmaz, sadece kanala push; watchdog ile yeniden kurulum |

---

## 12. Onay Sonrası İlk Adım

Faz 1: proje iskeleti kurulur, `npm run tauri dev` ile açılan, tema token'ları yüklü çalışır pencere teslim edilir. Ardından fazlar sırayla ilerler; her fazın sonunda build gate raporlanır.

---

## 13. Uygulama Durumu — 2026-09-07

Plan onaylandı ve baştan sona uygulandı. Aşağıdaki tablo, kodun bugünkü hâli.

| Faz | Durum | Not |
|---|---|---|
| 1 · Kurulum | ✅ | Tauri 2.11 + Vite 7.3 + React 19.2 + TS 5.9 strict + Tailwind 4.3; Inter `@fontsource-variable` ile bundle içinde |
| 2 · Native input | ✅ | `inject.rs` (scan-code SendInput), `hook.rs` (WH_*_LL + mesaj döngüsü), `keycodes.rs`, `timing.rs` |
| 3 · Veri + storage | ✅ | `model.rs`, `validate.rs`, `atomic_json.rs`, `history.rs`; TS tipleri birebir eş |
| 4 · Engine | ✅ | `control.rs` (atomic state machine + condvar), `executor.rs`, `engine.rs`, `scheduler.rs`, hotkey `matcher.rs` |
| 5 · UI mimarisi | ✅ | Custom title bar, animasyonlu sidebar, sayfa geçişleri, tek `MotionConfig` |
| 6 · Dashboard + liste | ✅ | Number ticker, empty state, sonner |
| 7 · Editor | ✅ | dnd-kit sıralama, palet, undo/redo (Ctrl+Z/Y/S), adım bazlı Test |
| 8 · Mouse | ✅ | SVG visualizer (5 bölge), CPS ↔ interval çift yönlü |
| 9 · Keyboard | ✅ | Tam TKL visualizer + modifier kombo |
| 10 · Recorder | ✅ | Canlı timeline, hareket eşiği, tık/tuş eşleştirme |
| 11 · Profiles | ✅ | Sınırlı renk paleti, aktif profil geçişi kısayolları yeniden bağlıyor |
| 12 · Settings | ✅ | 6 kategori, tema, animasyon kalitesi, hotkey çakışma kontrolü |
| 13 · Tray + autostart | ✅ | Open/Start/Pause/Stop/Exit, kapatınca tepsiye |
| 14 · Animasyon cilası | ✅ | `prefers-reduced-motion` + Low/Balanced/High tek noktadan CSS + JS token'larını kısıyor |
| 15 · Performans | ✅ | Hook event-driven, status 10 Hz throttle, `timeBeginPeriod` yalnız çalışırken |
| 16 · Test | ✅ | 86 Rust birim testi + clippy temiz; ayrıca `--ignored` ile gerçek imleç enjeksiyon testi |
| 17 · Paketleme | ✅ | NSIS `.exe` + `.msi` |

### Plandan bilinçli iki sapma

1. **`src-tauri/src/macro/` → `macros/`.** `macro` Rust'ta anahtar kelime; modül adı olamıyor.
2. **Ortam tablosu (§0) yanlıştı.** MSVC C++ araç seti ve Windows SDK kurulu değildi;
   bu oturumda `Microsoft.VisualStudio.Component.VC.Tools.x86.x64` +
   `Windows11SDK.26100` kuruldu. Ayrıca `cargo` Git Bash'ten çalışmıyor
   (GNU `link` MSVC linker'ını gölgeliyor) — PowerShell kullanılmalı.
