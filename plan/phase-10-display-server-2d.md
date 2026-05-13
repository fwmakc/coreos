# Этап 10 — Display Server: 2D Primitives & Text

## Цель
Добавить в Display Server 2D-примитивы (прямоугольники, скруглённые углы, границы, тени), текстовый рендеринг (все алфавиты), и базовые текстуры. После этого этапа Workspace умеет рендерить читаемый UI: кнопки, надписи, иконки.

## Язык и стек
- **Язык:** Rust
- **Ключевые зависимости:** `glyphon` (текстовый рендеринг на основе cosmic-text + wgpu), `etagere` (rectangle packing для texture atlas), `lyon` (tessellation векторных фигур, опционально для сложных форм)
- **Шейдеры:** WGSL (signed distance fields для скруглений, текстурный сэмплинг для глифов)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 9** — Display Server: Core (GPU device, queue, render pipeline, swapchain).

## Часть системы
**Level 3 — Display Server** [См. layer-8 §3.1–3.4, layer-1 §3, layer-11 §UI]

## Требования

### 8.1 2D-примитивы
- **Прямоугольник:** position, size, background color (solid или linear gradient).
- **Скруглённые углы (border radius):** рендеринг через signed distance field (SDF) в WGSL. Поддержка независимого радиуса для каждого угла.
- **Граница (border):** толщина, цвет, стиль (solid). Рендерится поверх фона через SDF.
- **Тень (box shadow):** offset, blur radius, spread radius, color. Рендерится через размытый прямоугольник (Gaussian blur approximation в шейдере).
- **Непрозрачность (opacity):** alpha-blending через `BlendState::ALPHA_BLENDING`.

### 8.2 Текстовый рендеринг
- **Шрифты:** системные шрифты хост-ОС + встроенный fallback (Noto Sans для всех алфавитов).
- **Rasterization:** `glyphon` использует `cosmic-text` для layout и `swash` для растеризации в атлас.
- **Атлас глифов:** динамический texture atlas (2048×2048 по умолчанию, расширяется при необходимости). Глифы кэшируются между кадрами.
- **Текстовые параметры:** размер, weight (400, 700), цвет, alignment (left/center/right), line height, max lines, overflow (ellipsis).
- **Кириллица:** обязательная поддержка с первого дня (не placeholder).
- **Эмодзи:** цветные эмодзи через цветной texture atlas (если шрифт поддерживает).

### 8.3 Текстуры
- Загрузка изображений: PNG, JPEG, WebP (через `image` crate).
- Texture atlas для UI-иконок и small images.
- Sampler configuration: linear filtering для UI, nearest для pixel-art.

### 8.4 Batch rendering
- Объединение примитивов с одинаковым материалом в один draw call.
- Instanced rendering для однотипных элементов (например, 100 кнопок — один mesh + instance buffer).
- Целевой бюджет: < 100 draw calls на типичный экран (Command Bar + 3 окна).

### 8.5 Coordinate system
- Логические пиксели (device-independent pixels, DIPs).
- Масштабирование через DPI factor от Host Shim.
- Origin (0, 0) — левый верхний угол surface.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Rounded rect | Скруглённые углы | Рендер → визуально скруглено |
| Text "Hello CORE" | Латиница | Рендер → читаемый текст |
| Text "Привет" | Кириллица | Рендер → читаемый текст |
| Border + shadow | Стилизация | Рендер → граница и тень видны |
| 100 buttons | Instancing | 100 кнопок → < 5 draw calls |
| Image display | PNG | Загрузить PNG → отображается |

## Интеграция с будущими этапами
- **Вход:** этап 7 (Core) — GPU device, queue, swapchain.
- **Выход:** 2D primitive API → этап 9 (Compositor) для компоновки окон.
- **Выход:** text rendering → этап 13 (Command Bar) для отображения текста.
- **Выход:** image textures → этап 14 (Project Manager) для иконок и превью.

## Критерии приёмки
- [ ] Рендерится прямоугольник с border radius 8px.
- [ ] Текст "Hello CORE" и "Привет, мир!" читаем, без артефактов.
- [ ] Граница 1px и тень 4px blur визуально корректны.
- [ ] 100 кнопок рендерятся за < 1 мс (GPU time).
- [ ] PNG изображение 512×512 загружается и отображается.
- [ ] Glyph atlas не переполняется при типичном использовании (текст Command Bar + 3 окна).

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Command Bar, окна
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Display Server §3.1–3.4
- [layer-11-developer-reference.md](../layers/layer-11-developer-reference.md) — UI описание
