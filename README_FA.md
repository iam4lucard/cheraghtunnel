<div align="center">
 
# 🕯️ CheraghTunnel (چراغ‌تونل)
 
**سامانه جامع مدیریت و استقرار تونل معکوس پیشرفته با کارایی فوق‌العاده بالا — نوشته‌شده با Rust**

[🌐 Read in English](README.md)
 
[![GitHub Release](https://img.shields.io/github/v/release/iam4lucard/cheraghtunnel?style=for-the-badge&logo=github&color=f59e0b)](https://github.com/iam4lucard/cheraghtunnel/releases/latest)
[![Build Status](https://img.shields.io/github/actions/workflow/status/iam4lucard/cheraghtunnel/release.yml?style=for-the-badge&logo=github-actions&label=CI)](https://github.com/iam4lucard/cheraghtunnel/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
 
<br/>
 
**چراغ‌تونل** یک پروژه امنیتی، یکپارچه و متن‌باز برای دور زدن محدودیت‌های شدید اینترنت و برقراری ارتباط سرور به سرور (معکوس) است. این سامانه شامل هسته قدرتمند کلاینت/سرور، موتور پروکسی چندپروتکله و پنل وب مدرن با رابط کاربری گلس‌مورفیک (Glassmorphism) است که همگی در قالب **یک باینری واحد استاتیک** بدون هیچ‌گونه وابستگی خارجی کامپایل شده‌اند.
 
<br/>
 
**`< 15 MB RAM`** &nbsp;•&nbsp; **`< 7 MB Binary`** &nbsp;•&nbsp; **`Zero Dependencies`** &nbsp;•&nbsp; **`Single Binary`**
 
</div>
 
---
 
## 📑 فهرست مطالب
 
- [ویژگی‌های کلیدی](#-ویژگی‌های-کلیدی)
- [پروتکل‌های انتقال (Transports)](#-پروتکل‌های-انتقال-transports)
- [پنل مدیریت تحت وب](#-پنل-مدیریت-تحت-وب)
- [نصب سریع](#-نصب-سریع)
- [راهنمای استفاده CLI](#-راهنمای-استفاده-cli)
- [مکانیسم امنیتی و استتار](#-مکانیسم-امنیتی-و-استتار)
- [توسعه و کامپایل از سورس](#-توسعه-و-کامپایل-از-سورس)
- [لایسنس](#-لایسنس)
 
---
 
## ✨ ویژگی‌های کلیدی
 
* 🚀 **۱۶ پروتکل انتقال متنوع:** از TCP ساده تا لایه‌های WebRTC ،Reality ،Pulsar و پروتکل‌های خلاقانه اختصاصی مانند Spectre، Oracle، Vortex و Nirvana جهت عبور از دیوارهای آتش مختلف بر اساس شرایط شبکه.
* ⚡ **هندشیک بدون تاخیر (0-RTT):** ارسال کلیدهای امنیتی و داده‌ها به صورت همزمان در اولین پکت ارسالی و حذف تاخیر RTT اولیه اتصال در پروتکل‌های `Spectre` ،`Mirage` ،`Nirvana` و `Beam`.
* 🔒 **تصادفی‌سازی حجم پکت‌ها (Dynamic Packet Padding):** تزریق بایت‌های تصادفی (0-256 بایت) به فریم‌ها جهت خنثی‌سازی کامل آنالیز الگوی سایز پکت توسط DPI.
* 📡 **ترافیک کاذب دوره ای (Dummy Chaffing):** ارسال پکت‌های کاذب در زمان‌های سکوت کانکشن جهت بهم زدن الگوی رفتاری شبکه.
* 🛡️ **شبیه‌سازی ECH (Encrypted ClientHello):** افزوده شدن اکستنشن `0xfe0d` و مرتب‌سازی اکستنشن‌های TLS مرورگرهای مدرن در Spectre و Mirage.
* 🔀 **توزیع چندمسیری پکت‌ها (Multipath IP Spraying):** تقسیم و ارسال موازی پکت‌ها روی چندین IP سرور خارج به صورت همزمان.
* 🔄 **پرش پورت پویا (Dynamic Port Hopping):** تغییر زمان‌بندی‌شده و رمزنگاری‌شده‌ی پورت کنترلر (هر ۵ دقیقه) روی سرور ایران.
* 🔄 **سوئیچ خودکار سرور خارج (Node Failover):** مانیتورینگ سلامت نودها و انتقال آنی ترافیک تانل‌های فعال روی سرورهای رزرو (Backup Nodes).
* ⚙️ **استقرار خودکار تک‌کلیکی (SSH Auto-Deploy):** پیکربندی و اجرای خودکار کلاینت روی سرور خارج به کمک اتصال SSH.
* ⏳ **محدودیت حجم، سرعت و انقضا:** امکان تنظیم سقف حجم (GB)، سقف سرعت (KB/s) و تاریخ انقضا برای هر تانل.
* 📊 **مانیتورینگ زنده وب‌سوکت (WebSocket Telemetry):** نمایش لحظه‌ای پینگ، Loss، سرعت مصرفی و سخت‌افزار سرور روی وب‌سوکت.
 
---
 
## 🔌 پروتکل‌های انتقال (Transports)
 
| پروفایل | شناسه فنی | لایه انتقال | توضیحات | بهترین کاربرد |
|:---:|:---:|:---:|:---|:---|
| 🔵 **Beam** | `tcpmux` | TCP (0-RTT) | ارتباط ساده و بسیار سریع TCP موازی با احراز هویت بدون تاخیر | عمومی و پرسرعت |
| 🟢 **Aura** | `httpmux` | HTTP | شبیه‌سازی ترافیک و هدرهای معمولی وب HTTP/1.1 | شبکه‌های بسیار محدود |
| 🟡 **Nova** | `httpsmux` | HTTPS | انتقال تماماً رمزنگاری‌شده با TLS معتبر و کامل | امنیت حداکثری |
| 🟣 **Glimmer** | `wsmux` | WebSocket | وب‌سوکت ساده جهت عبور از شبکه‌های توزیع محتوا | عبور از CDN |
| 🔴 **Beacon** | `wssmux` | WSS | وب‌سوکت امن با لایه TLS — سازگار با کلودفلر | CDN با امنیت بالا |
| ⚡ **Flash** | `kcpmux` | KCP/UDP | پروتکل سرعت بالای گیمینگ مبتنی بر UDP | بازی‌های آنلاین و پینگ پایین |
| 🌊 **Ray** | `rawmux` | Raw UDP | ارتباط مستقیم KCP با کمترین اورهد در سطح سوکت | ارتباطات بلادرنگ |
| ⚛️ **Photon** | `quantummux` | TCP+FEC | ترکیب نوآورانه‌ی TCP و KCP با تصحیح خطا بدون استفاده از UDP | دور زدن فیلترینگ UDP |
| 🏮 **Lantern** | `tunmux` | TUN L2/L3 | تونل سطح شبکه با ساخت اینترفیس مجازی سیستم‌عامل | انتقال کل ترافیک سیستم |
| 🌫️ **Mirage** | `realitymux` | Reality TLS (0-RTT) | جعل گواهینامه TLS 1.3 سایت‌های معتبر با هندشیک بدون تاخیر | عبور از دیوارهای آتش هوشمند DPI |
| 👼 **Halo** | `webrtcmux` | WebRTC | شبیه‌سازی پکت‌ها مشابه تماس‌های صوتی/تصویری اینترنتی | دور زدن DPI سخت‌گیرانه |
| 💫 **Pulsar** | `pulsar` | QUIC/UDP | پروتکل پالس‌محور و پرسرعت بر پایه QUIC با قابلیت کنترل جریان | شبکه‌های دارای نوسان و نویز |
| 🔮 **Oracle** | `oracle` | DNS/UDP | شبیه‌سازی کوئری‌های معتبر DNS با افزونه‌های EDNS0 روی پورت ۵۳ | دور زدن فیلترینگ شدید UDP |
| 🌀 **Vortex** | `vortex` | Steam/UDP | شبیه‌سازی پکت‌های پینگ و کوئری سرور بازی‌های آنلاین (Source Engine) | گیمینگ با QoS اولویت بالا |
| 🕉️ **Nirvana** | `nirvana` | HTTP/TCP (0-RTT) | شبیه‌سازی هوشمند درخواست‌های وب POST با متد Chunked و رمزنگاری XOR | استتار کامل ترافیک TCP با سرعت بالا |
| 👻 **Spectre** | `spectre` | Multipath TLS (0-RTT) | پروتکل فوق‌سریع بازی با قابلیت 0-RTT Reality TLS و مالتی‌پس (Multipath Spraying) | گیمینگ با پینگ کاملاً تخت و ثابت |
 
---
 
## 🎨 پنل مدیریت تحت وب
 
پنل وب چراغ‌تونل امکان مدیریت بدون نیاز به خط فرمان را با امکانات زیر فراهم می‌سازد:
* **داشبورد مانیتورینگ زنده:** استریم آنلاین مصرف منابع سرور (سی‌پی‌یو و رم) و وضعیت اتصالات بر روی وب‌سوکت.
* **مدیریت نودها (Iran/Kharej):** تعریف سرورها با نقش‌های مختلف و ثبت اطلاعات SSH جهت استقرار خودکار کلاینت‌ها.
* **ایجاد و ویرایش تانل:** امکان تغییر لحظه‌ای پروتکل، پورت‌ها، ECH، Dynamic Padding، Chaffing، Multipath و پرش پورت (Port Hopping).
* **بخش پشتیبان‌گیری:** دانلود نسخه پشتیبان دیتابیس و آپلود آنی جهت بازیابی کل سیستم در سرور جدید.
 
---
 
## 🚀 نصب سریع
 
### روش اول: اسکریپت نصب خودکار (توصیه‌شده)
 
برای نصب خودکار پنل مدیریتی، دستور زیر را به عنوان کاربر `root` روی سرور ایران خود اجرا کنید:
 
```bash
curl -sSf https://raw.githubusercontent.com/iam4lucard/cheraghtunnel/main/install.sh | bash
```
 
این اسکریپت مراحل نصب را به صورت تعاملی پیش برده و موارد زیر را از شما می‌پرسد:
* **پورت پنل وب:** پورتی که پنل روی آن بالا می‌آید (پیش‌فرض: `8000`).
* **نام کاربری و رمز عبور:** مشخصات ورود ادمین برای قفل امنیتی پنل.
 
پس از پایان، سرویس پنل وب به صورت یک سرویس `systemd` به نام `cheraghtunnel` ثبت شده و در پس‌زمینه اجرا می‌شود.
 
### روش دوم: دانلود مستقیم باینری آماده
 
شما می‌توانید مستقیماً باینری کامپایل‌شده‌ی آخرین نسخه را برای پلتفرم خود دریافت کنید:
 
```bash
# نسخه‌ی لینوکس (amd64)
curl -sSfL -o /usr/local/bin/cheraghtunnel \
  https://github.com/iam4lucard/cheraghtunnel/releases/latest/download/cheraghtunnel-linux-amd64
chmod +x /usr/local/bin/cheraghtunnel
 
# نسخه‌ی لینوکس (arm64)
curl -sSfL -o /usr/local/bin/cheraghtunnel \
  https://github.com/iam4lucard/cheraghtunnel/releases/latest/download/cheraghtunnel-linux-arm64
chmod +x /usr/local/bin/cheraghtunnel
```
 
---
 
## 💻 راهنمای استفاده CLI
 
در صورتی که می‌خواهید هسته تونل را بدون پنل وب و به صورت دستی در خط فرمان اجرا کنید:
 
### ۱. اجرای پنل وب با خط فرمان
 
```bash
cheraghtunnel panel --port 8000 --db-path /var/lib/cheraghtunnel/cheraghtunnel.db
```
 
### ۲. اجرای سرور (ایران)
 
```bash
cheraghtunnel server \
  --control-port 8090 \
  --public-port 443 \
  --token SECRET_TOKEN \
  --protocol spectre \
  --decoy https://www.microsoft.com \
  --port-hopping
```
 
### ۳. اجرای کلاینت (خارج)
 
```bash
cheraghtunnel client \
  --server-ip 62.60.202.4 \
  --control-port 8090 \
  --public-port 443 \
  --local-service 127.0.0.1:1080 \
  --token SECRET_TOKEN \
  --protocol spectre \
  --tunnel-id 1 \
  --port-hopping
```
 
---
 
## 🔒 مکانیسم امنیتی و استتار
 
چراغ‌تونل امنیت لایه‌ای و حریم خصوصی را با ویژگی‌های پیشرفته تضمین می‌کند:
* **مقایسه‌های زمان‌ثابت (Constant-Time Operations):** جلوگیری از حملات کانال جانبی تحلیل زمان (Timing Attacks) در زمان احراز هویت توکن‌ها.
* **پدافند غیرعامل در برابر اسکن‌های فعال (Decoy):** در صورتی که هرگونه درخواست غیرمجاز به پورت کنترلر ارسال شود، سیستم به طور خودکار پاسخ‌های فریبنده یا ریدایرکت به وب‌سایت‌های معتبر را شبیه‌سازی می‌کند.
* **Rate Limiting هوشمند:** محافظت از پنل وب در برابر حملات Brute-force با محدودسازی تعداد ورود‌های ناموفق.
* **مکانیزم پاکسازی آنی پورت‌ها:** استفاده از آپشن‌های سوکت سیستم‌عامل (`SO_REUSEADDR` و `SO_REUSEPORT`) جهت آزادسازی فوری پورت‌ها هنگام تغییر وضعیت تانل بدون اشغال شدن پورت.
 
---
 
## 🛠 توسعه و کامپایل از سورس
 
### پیش‌نیازها
* [Rust و Cargo](https://rustup.rs/) نسخه 1.75 یا بالاتر
* کتابخانه توسعه SQLite (`libsqlite3-dev` در توزیع‌های دبیان/اوبونتو)
 
### مراحل ساخت
 
```bash
# کلون ریپازیتوری
git clone https://github.com/iam4lucard/cheraghtunnel.git
cd cheraghtunnel
 
# بیلد نسخه نهایی (Release)
cargo build --release
 
# اجرای فایل خروجی
./target/release/cheraghtunnel panel --port 8000
```
 
---
 
## 📜 لایسنس
 
این پروژه تحت لایسنس **[MIT](LICENSE)** توسعه داده می‌شود و استفاده، ویرایش و توزیع آن به هر شکلی کاملاً آزاد و رایگان است.
 
<div align="center">
 
**ساخته‌شده با ❤️ و قدرت Rust**
 
[🐛 گزارش مشکلات](https://github.com/iam4lucard/cheraghtunnel/issues) &nbsp;•&nbsp; [💡 ثبت ایده جدید](https://github.com/iam4lucard/cheraghtunnel/issues) &nbsp;•&nbsp; [📦 نسخه‌های منتشر شده](https://github.com/iam4lucard/cheraghtunnel/releases)
 
</div>
