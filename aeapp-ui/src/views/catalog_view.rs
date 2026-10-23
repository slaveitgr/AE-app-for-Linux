//! Public EA product discovery. Catalog artwork never implies ownership or membership.
use crate::{shell, AeAppEguiApp, PageType};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, TextureHandle, Ui};
use serde::Deserialize;
use std::{
    path::{Component, Path},
    sync::Arc,
};

const RED: Color32 = Color32::from_rgb(255, 71, 76);
const COPY: Color32 = Color32::from_rgb(178, 177, 191);
const PLAY_URL: &str = "https://www.ea.com/ea-play";

#[derive(Clone, Deserialize)]
struct CatalogRecord {
    slug: String,
    #[serde(default)]
    filename: String,
    #[serde(alias = "url")]
    source_url: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    image_file: Option<String>,
    #[serde(default)]
    price_label: Option<String>,
    #[serde(default)]
    release_year: Option<u16>,
}
#[derive(Deserialize)]
struct ShopCatalog {
    version: u32,
    products: Vec<CatalogRecord>,
}
#[derive(Clone)]
struct PublicProduct {
    record: CatalogRecord,
    texture: Option<TextureHandle>,
    verified_metadata: bool,
}
#[derive(Clone, Default)]
struct PublicCatalog {
    products: Vec<PublicProduct>,
}

fn official_url(value: &str) -> bool {
    reqwest::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && url.port_or_known_default() == Some(443)
            && matches!(
                url.host_str(),
                Some("ea.com" | "www.ea.com" | "store.ea.com")
            )
    })
}
fn relative_image_path(relative: &str) -> Option<&Path> {
    let path = Path::new(relative);
    (!relative.is_empty() && path.components().all(|part| matches!(part, Component::Normal(_))))
        .then_some(path)
}
fn local_picture(ui: &Ui, base: &Path, relative: &str, slug: &str) -> Option<TextureHandle> {
    let path = relative_image_path(relative)?;
    let root = base.canonicalize().ok()?;
    let file = base.join(path).canonicalize().ok()?;
    if !file.starts_with(root) {
        return None;
    }
    let image = image::load_from_memory(&std::fs::read(file).ok()?)
        .ok()?
        .thumbnail(768, 768)
        .into_rgba8();
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    );
    Some(ui.ctx().load_texture(
        format!("Public EA catalog: {slug}"),
        image,
        egui::TextureOptions::LINEAR,
    ))
}
fn catalog(ui: &Ui) -> PublicCatalog {
    let directory = crate::local_asset_directory();
    let id = egui::Id::new(("aeapp-public-product-catalog", &directory));
    if let Some(catalog) = ui.ctx().data(|data| data.get_temp::<PublicCatalog>(id)) {
        return catalog;
    }
    let mut catalog = PublicCatalog::default();
    if let Some(directory) = directory {
        let shop_dir = directory.join("catalog");
        let shop = std::fs::read(shop_dir.join("products.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<ShopCatalog>(&bytes).ok())
            .filter(|shop| shop.version == 1 && !shop.products.is_empty());
        let (base, records, verified_metadata) = if let Some(shop) = shop {
            (shop_dir, shop.products, true)
        } else {
            let covers = directory.join("covers");
            let records = std::fs::read(covers.join("catalog.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Vec<CatalogRecord>>(&bytes).ok())
                .unwrap_or_default();
            (covers, records, false)
        };
        for record in records {
            if !official_url(&record.source_url) {
                continue;
            }
            let filename = if verified_metadata {
                record.image_file.as_deref().unwrap_or("")
            } else {
                &record.filename
            };
            let texture = local_picture(ui, &base, filename, &record.slug);
            catalog.products.push(PublicProduct {
                record,
                texture,
                verified_metadata,
            });
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, catalog.clone()));
    catalog
}
fn title(app: &AeAppEguiApp, product: &PublicProduct) -> String {
    if product.verified_metadata {
        if let Some(title) = product.record.title.as_ref().filter(|title| !title.trim().is_empty())
        {
            return title.clone();
        }
    }
    if let Some(game) = app.games.get(&product.record.slug) {
        return game.name.clone();
    }
    match product.record.slug.as_str() {
        "need-for-speed-payback" => "Need for Speed Payback".to_owned(),
        "need-for-speed-heat" => "Need for Speed Heat".to_owned(),
        "the-sims-4" => "The Sims 4".to_owned(),
        "star-wars-jedi-survivor" => "STAR WARS Jedi: Survivor".to_owned(),
        "titanfall-2" => "Titanfall 2".to_owned(),
        "mass-effect-legendary" => "Mass Effect Legendary Edition".to_owned(),
        "it-takes-two" => "It Takes Two".to_owned(),
        slug => slug
            .split('-')
            .map(|word| {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}
fn text(ui: &Ui, at: Pos2, copy: &str, font: FontId, color: Color32, width: f32) {
    let galley = ui.painter().layout(copy.to_owned(), font, color, width.max(1.0));
    ui.painter().galley(at, galley, color);
}
fn one_line(ui: &Ui, rect: Rect, copy: &str, font: FontId, color: Color32) {
    let mut job = egui::text::LayoutJob::simple_singleline(copy.to_owned(), font, color);
    job.wrap.max_width = rect.width();
    job.wrap.max_rows = 1;
    job.wrap.overflow_character = Some('…');
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    ui.painter().with_clip_rect(rect).galley(rect.min, galley, color);
}
fn art(ui: &Ui, rect: Rect, product: &PublicProduct, rounding: f32) {
    if let Some(texture) = &product.texture {
        let source = texture.aspect_ratio();
        let target = rect.width() / rect.height();
        let uv = if source > target {
            let width = target / source;
            Rect::from_min_max(
                pos2((1.0 - width) * 0.5, 0.0),
                pos2((1.0 + width) * 0.5, 1.0),
            )
        } else {
            let height = source / target;
            Rect::from_min_max(
                pos2(0.0, (1.0 - height) * 0.5),
                pos2(1.0, (1.0 + height) * 0.5),
            )
        };
        egui::Image::new((texture.id(), texture.size_vec2()))
            .uv(uv)
            .rounding(rounding)
            .paint_at(ui, rect);
    } else {
        ui.painter().rect_filled(rect, rounding, shell::SURFACE);
        ui.painter().circle_stroke(
            rect.center(),
            rect.width().min(rect.height()) * 0.22,
            Stroke::new(2.0, shell::BORDER),
        );
    }
}
fn action(ui: &mut Ui, rect: Rect, label: &str, preview: bool, primary: bool, url: &str) {
    let clicked = ui
        .allocate_ui_at_rect(rect, |ui| {
            ui.add_enabled(
                !preview,
                egui::Button::new(egui::RichText::new(label).font(shell::bold(12.0)))
                    .fill(if primary {
                        crate::ACCENT_COLOR
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(if primary {
                        Stroke::NONE
                    } else {
                        Stroke::new(1.0, Color32::from_rgb(118, 119, 139))
                    })
                    .rounding(4.0)
                    .min_size(rect.size()),
            )
            .on_disabled_hover_text("External EA links are disabled in local preview.")
            .clicked()
        })
        .inner;
    if clicked && official_url(url) {
        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
    }
}
fn product_card(
    app: &AeAppEguiApp,
    ui: &mut Ui,
    rect: Rect,
    product: &PublicProduct,
    context: &str,
    trial: bool,
) {
    let cover = Rect::from_min_size(rect.min, vec2(rect.width(), rect.width()));
    art(ui, cover, product, 16.0);
    let response = ui.interact(
        cover,
        ui.id().with((context, &product.record.slug)),
        Sense::click(),
    );
    if response.hovered() && !app.is_preview() {
        ui.painter().rect_stroke(
            cover.expand(2.0),
            17.0,
            Stroke::new(2.0, crate::ACCENT_COLOR),
        );
    }
    if response.clicked() && !app.is_preview() {
        let url = if trial {
            PLAY_URL
        } else {
            &product.record.source_url
        };
        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
    }
    response.on_hover_text(if app.is_preview() {
        "Public EA product artwork. External store links are disabled in local preview."
    } else if trial {
        "Check current game trials and eligibility on EA.com."
    } else {
        "Open this public product on EA.com for current prices and availability."
    });
    let title_rect = Rect::from_min_size(
        cover.left_bottom() + vec2(0.0, 16.0),
        vec2(rect.width(), 18.0),
    );
    let price = product
        .verified_metadata
        .then_some(product.record.price_label.as_deref())
        .flatten()
        .filter(|price| !price.trim().is_empty());
    one_line(
        ui,
        title_rect,
        &title(app, product),
        shell::bold(12.0),
        shell::TEXT,
    );
    one_line(
        ui,
        Rect::from_min_size(
            cover.left_bottom() + vec2(0.0, 41.0),
            vec2(rect.width(), 20.0),
        ),
        if trial {
            "Check trial availability"
        } else {
            price.unwrap_or("View on EA.com")
        },
        FontId::proportional(12.0),
        COPY,
    );
}
fn product_grid(app: &AeAppEguiApp, ui: &mut Ui, products: &[PublicProduct], context: &str) {
    let width = ui.available_width();
    let columns = ((width + 24.0) / 264.0).floor().max(1.0) as usize;
    let card_width = (width - (columns - 1) as f32 * 24.0) / columns as f32;
    for (index, row) in products.chunks(columns).enumerate() {
        if index > 0 {
            ui.add_space(32.0);
        }
        let row_rect = ui.allocate_exact_size(vec2(width, card_width + 64.0), Sense::hover()).0;
        for (column, product) in row.iter().enumerate() {
            let rect = Rect::from_min_size(
                row_rect.min + vec2(column as f32 * (card_width + 24.0), 0.0),
                vec2(card_width, card_width + 64.0),
            );
            if ui.is_rect_visible(rect) {
                product_card(app, ui, rect, product, context, false);
            }
        }
    }
}
fn unavailable_filter(ui: &mut Ui, rect: Rect, title: &str, reason: &str) {
    ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_rgb(78, 79, 99)));
    text(
        ui,
        rect.min + vec2(12.0, 6.0),
        title,
        FontId::proportional(14.0),
        COPY,
        rect.width() - 36.0,
    );
    let center = rect.right_center() - vec2(16.0, 0.0);
    ui.painter().line_segment(
        [center + vec2(-4.0, -2.0), center + vec2(0.0, 2.0)],
        Stroke::new(1.5, COPY),
    );
    ui.painter().line_segment(
        [center + vec2(0.0, 2.0), center + vec2(4.0, -2.0)],
        Stroke::new(1.5, COPY),
    );
    let response = ui.interact(
        rect,
        ui.id().with(("catalog-filter", title)),
        Sense::hover(),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, false, title));
    response.on_hover_text(reason);
}

/// Show a single row of public releases with verified years and locally cached art.
/// A false return leaves the caller's layout untouched so it can show a library fallback.
pub fn home_products(app: &mut AeAppEguiApp, ui: &mut Ui) -> bool {
    let mut products: Vec<PublicProduct> = catalog(ui)
        .products
        .into_iter()
        .filter(|product| {
            product.verified_metadata
                && product.record.release_year.is_some()
                && product.texture.is_some()
        })
        .collect();
    if products.is_empty() {
        return false;
    }
    products.sort_by(|a, b| {
        b.record
            .release_year
            .cmp(&a.record.release_year)
            .then_with(|| title(app, a).to_lowercase().cmp(&title(app, b).to_lowercase()))
    });
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.label(egui::RichText::new("New & recent releases").font(shell::bold(16.0)));
    ui.add_space(32.0);
    let width = ui.available_width();
    let columns = (((width + 24.0) / 264.0).floor().max(1.0) as usize).min(5);
    let card_width = (width - (columns - 1) as f32 * 24.0) / columns as f32;
    let row = ui.allocate_exact_size(vec2(width, card_width + 64.0), Sense::hover()).0;
    for (index, product) in products.iter().take(columns).enumerate() {
        let card = Rect::from_min_size(
            row.min + vec2(index as f32 * (card_width + 24.0), 0.0),
            vec2(card_width, card_width + 64.0),
        );
        product_card(app, ui, card, product, "home-public-releases", false);
    }
    true
}

pub fn browse(app: &mut AeAppEguiApp, ui: &mut Ui) {
    ui.spacing_mut().item_spacing.y = 0.0;
    let catalog = catalog(ui);
    let width = ui.available_width();
    let header = ui.allocate_exact_size(vec2(width, 193.0), Sense::hover()).0;
    text(
        ui,
        header.min,
        "Browse",
        shell::display(32.0),
        shell::TEXT,
        width,
    );
    let tab_id = egui::Id::new("public-catalog-featured-tab");
    let mut featured = ui.ctx().data(|data| data.get_temp::<bool>(tab_id)).unwrap_or(false);
    let tabs = Rect::from_min_size(header.min + vec2(0.0, 45.0), vec2(width, 34.0));
    ui.allocate_ui_at_rect(tabs, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for (label, selected) in [("All games", false), ("Featured games", true)] {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(label).font(shell::bold(16.0)).color(
                                if featured == selected {
                                    Color32::from_rgb(20, 23, 36)
                                } else {
                                    shell::TEXT
                                },
                            ),
                        )
                        .fill(if featured == selected {
                            Color32::WHITE
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::NONE)
                        .rounding(8.0)
                        .min_size(vec2(0.0, 34.0)),
                    )
                    .on_hover_text(if selected {
                        "A local selection of the available public EA products."
                    } else {
                        "All products in the local public catalog."
                    })
                    .clicked()
                {
                    featured = selected;
                }
            }
        });
    });
    ui.ctx().data_mut(|data| data.insert_temp(tab_id, featured));
    ui.painter().line_segment(
        [
            header.min + vec2(0.0, 104.0),
            header.min + vec2(width, 104.0),
        ],
        Stroke::new(1.0, shell::BORDER),
    );
    let mut x = 0.0;
    for (label, filter_width) in [
        ("Genres", 112.0),
        ("Included in", 140.0),
        ("Players", 112.0),
        ("Languages", 140.0),
    ] {
        unavailable_filter(
            ui,
            Rect::from_min_size(header.min + vec2(x, 121.0), vec2(filter_width, 32.0)),
            label,
            "This metadata is not available in the local public catalog.",
        );
        x += filter_width + 16.0;
    }
    let sort_id = egui::Id::new("public-catalog-sort");
    let mut sort = ui.ctx().data(|data| data.get_temp::<usize>(sort_id)).unwrap_or(0);
    if width > x + 160.0 {
        let sort_rect =
            Rect::from_min_size(header.right_top() + vec2(-152.0, 121.0), vec2(152.0, 32.0));
        if catalog
            .products
            .iter()
            .any(|product| product.verified_metadata && product.record.release_year.is_some())
        {
            ui.allocate_ui_at_rect(sort_rect, |ui| {
                ui.spacing_mut().interact_size.y = 32.0;
                egui::ComboBox::from_id_source("public-catalog-sort-menu")
                    .width(152.0).selected_text(if sort == 2 { "Name" } else { "Release date" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut sort, 0, "Newest first");
                        ui.selectable_value(&mut sort, 1, "Oldest first");
                        ui.selectable_value(&mut sort, 2, "Name (A–Z)");
                    }).response.on_hover_text("Sort by the release year published in the EA catalog. Dates without a published year appear last.");
            });
        } else {
            unavailable_filter(
                ui,
                sort_rect,
                "Release date",
                "Release dates are not available in the local public catalog.",
            );
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(sort_id, sort));
    ui.advance_cursor_after_rect(header);
    let search = shell::catalog_search(ui.ctx()).trim().to_lowercase();
    let mut products: Vec<PublicProduct> = catalog
        .products
        .iter()
        .filter(|product| title(app, product).to_lowercase().contains(&search))
        .cloned()
        .collect();
    if sort == 2 {
        products.sort_by_cached_key(|product| title(app, product).to_lowercase());
    } else if products
        .iter()
        .any(|product| product.verified_metadata && product.record.release_year.is_some())
    {
        products.sort_by(|a, b| {
            let year_order = if sort == 1 {
                a.record
                    .release_year
                    .unwrap_or(u16::MAX)
                    .cmp(&b.record.release_year.unwrap_or(u16::MAX))
            } else {
                b.record.release_year.cmp(&a.record.release_year)
            };
            year_order.then_with(|| title(app, a).to_lowercase().cmp(&title(app, b).to_lowercase()))
        });
    }
    if featured {
        products.truncate(5);
    }
    egui::ScrollArea::vertical().id_source(("public-catalog-products", featured)).auto_shrink(false).show(ui, |ui| {
        if products.is_empty() {
            if catalog.products.is_empty() {
                shell::empty(ui, "Explore EA games", "Local public catalog artwork is not available yet. Browse current products on EA.com.");
                let rect = ui.allocate_exact_size(vec2(180.0, 32.0), Sense::hover()).0;
                action(ui, rect, "OPEN EA GAMES", app.is_preview(), true, "https://www.ea.com/games/library");
            } else {
                shell::empty(ui, "No matching games", "Try another search.");
            }
        } else {
            product_grid(app, ui, &products, "public-catalog");
            ui.add_space(24.0);
            ui.label(egui::RichText::new("Public EA catalog. Confirm current prices and availability on EA.com.").size(12.0).color(COPY));
        }
        ui.add_space(48.0);
    });
}

#[derive(Clone)]
struct LogoBytes(Option<Arc<[u8]>>);
fn play_logo(ui: &Ui, rect: Rect) {
    let id = egui::Id::new("public-ea-play-logo");
    let bytes = ui.ctx().data(|data| data.get_temp::<LogoBytes>(id)).unwrap_or_else(|| {
        let bytes = crate::local_asset_directory()
            .and_then(|directory| {
                std::fs::read_to_string(directory.join("icons/EA_PLAY_MEDALLION.svg")).ok()
            })
            .map(|svg| {
                Arc::<[u8]>::from(svg.replacen("<svg ", "<svg fill=\"#ff474c\" ", 1).into_bytes())
            });
        let cached = LogoBytes(bytes);
        ui.ctx().data_mut(|data| data.insert_temp(id, cached.clone()));
        cached
    });
    if let Some(bytes) = bytes.0 {
        let symbol = Rect::from_min_size(rect.min, vec2(rect.height(), rect.height()));
        egui::Image::from_bytes("bytes://aeapp-local-ea-play-medallion.svg", bytes)
            .paint_at(ui, symbol);
        text(
            ui,
            rect.min + vec2(rect.height() + 3.0, 6.0),
            "Play",
            shell::display(rect.height() * 0.73),
            RED,
            rect.width() - rect.height(),
        );
    } else {
        text(
            ui,
            rect.min + vec2(0.0, 6.0),
            "EA Play",
            shell::display(32.0),
            RED,
            rect.width(),
        );
    }
}
fn mosaic(ui: &mut Ui, rect: Rect, products: &[PublicProduct]) {
    ui.painter().rect_filled(rect, 16.0, shell::SURFACE);
    if !products.is_empty() {
        let painter = ui.painter().with_clip_rect(rect);
        let tile = rect.width() / 6.0;
        for row in 0..3 {
            for column in 0..7 {
                let product = &products[(row * 7 + column) % products.len()];
                let image_rect = Rect::from_min_size(
                    rect.min
                        + vec2(
                            column as f32 * (tile + 10.0) - tile * 0.35,
                            row as f32 * (tile + 10.0) - tile * 0.3,
                        ),
                    vec2(tile, tile),
                );
                let mut image_ui = ui.child_ui(rect, *ui.layout(), None);
                image_ui.set_clip_rect(ui.clip_rect().intersect(rect));
                art(&image_ui, image_rect, product, 12.0);
            }
        }
        painter.rect_filled(rect, 16.0, Color32::from_rgba_unmultiplied(30, 30, 48, 205));
    }
}
fn trial_row(app: &AeAppEguiApp, ui: &mut Ui, products: &[PublicProduct]) {
    let width = ui.available_width();
    let columns = 3.min(products.len());
    if columns == 0 {
        return;
    }
    let gap = 24.0;
    let card_width = ((width - 4.0 * gap) / 5.0).clamp(150.0, 250.0);
    let image_height = card_width;
    let row = ui.allocate_exact_size(vec2(width, image_height + 68.0), Sense::hover()).0;
    for (index, product) in products.iter().take(3).enumerate() {
        let rect = Rect::from_min_size(
            row.min + vec2(index as f32 * (card_width + gap), 0.0),
            vec2(card_width, image_height),
        );
        art(ui, rect, product, 16.0);
        let response = ui.interact(
            rect,
            ui.id().with(("ea-play-trial-discovery", index)),
            Sense::click(),
        );
        if response.clicked() && !app.is_preview() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(PLAY_URL));
        }
        response.on_hover_text(
            "View current trials on EA.com. Trial eligibility for this game is not loaded.",
        );
        one_line(
            ui,
            Rect::from_min_size(rect.left_bottom() + vec2(0.0, 16.0), vec2(card_width, 18.0)),
            &title(app, product),
            shell::bold(12.0),
            shell::TEXT,
        );
        one_line(
            ui,
            Rect::from_min_size(rect.left_bottom() + vec2(0.0, 41.0), vec2(card_width, 20.0)),
            "Check trial availability",
            FontId::proportional(12.0),
            COPY,
        );
    }
}
fn comparison(app: &AeAppEguiApp, ui: &mut Ui) {
    let width = ui.available_width();
    let rect = ui.allocate_exact_size(vec2(width, 470.0), Sense::hover()).0;
    text(
        ui,
        rect.min,
        "Find the membership for you",
        shell::display(28.0),
        shell::TEXT,
        width,
    );
    let label_width = width * 0.4;
    let plan_width = (width - label_width) * 0.5;
    for (index, label) in ["EA Play", "EA Play Pro"].iter().enumerate() {
        text(
            ui,
            rect.min + vec2(label_width + plan_width * index as f32 + 24.0, 70.0),
            label,
            shell::display(24.0),
            RED,
            plan_width - 32.0,
        );
    }
    for (row, (feature, play, pro)) in [
        (
            "Selected new releases",
            "Trials lasting up to 10 hours",
            "Full access to selected premium editions",
        ),
        (
            "EA game collection",
            "Collection access during membership",
            "Collection access during membership",
        ),
        (
            "In-game extras",
            "Selected member rewards",
            "Selected member rewards",
        ),
        (
            "EA digital purchases",
            "10% member discount",
            "10% member discount",
        ),
    ]
    .iter()
    .enumerate()
    {
        let y = 126.0 + row as f32 * 62.0;
        ui.painter().line_segment(
            [
                rect.min + vec2(0.0, y - 12.0),
                rect.min + vec2(width, y - 12.0),
            ],
            Stroke::new(1.0, shell::BORDER),
        );
        text(
            ui,
            rect.min + vec2(0.0, y),
            feature,
            shell::bold(14.0),
            shell::TEXT,
            label_width - 24.0,
        );
        text(
            ui,
            rect.min + vec2(label_width + 24.0, y),
            play,
            FontId::proportional(14.0),
            COPY,
            plan_width - 40.0,
        );
        text(
            ui,
            rect.min + vec2(label_width + plan_width + 24.0, y),
            pro,
            FontId::proportional(14.0),
            COPY,
            plan_width - 40.0,
        );
    }
    action(
        ui,
        Rect::from_min_size(
            rect.min + vec2(label_width + 24.0, 384.0),
            vec2((plan_width - 40.0).min(220.0), 32.0),
        ),
        "VIEW EA PLAY",
        app.is_preview(),
        true,
        PLAY_URL,
    );
    action(
        ui,
        Rect::from_min_size(
            rect.min + vec2(label_width + plan_width + 24.0, 384.0),
            vec2((plan_width - 40.0).min(220.0), 32.0),
        ),
        "VIEW EA PLAY PRO",
        app.is_preview(),
        true,
        PLAY_URL,
    );
    text(
        ui,
        rect.min + vec2(0.0, 442.0),
        "Benefits, eligibility, supported platforms and current prices are confirmed on EA.com.",
        FontId::proportional(12.0),
        COPY,
        width,
    );
    ui.advance_cursor_after_rect(rect);
}

pub fn ea_play(app: &mut AeAppEguiApp, ui: &mut Ui) {
    ui.spacing_mut().item_spacing.y = 0.0;
    let catalog = catalog(ui);
    egui::ScrollArea::vertical()
        .id_source("ea-play-discovery")
        .auto_shrink(false)
        .show(ui, |ui| {
            let width = ui.available_width();
            let hero = ui.allocate_exact_size(vec2(width, 440.0), Sense::hover()).0;
            mosaic(ui, hero, &catalog.products);
            play_logo(
                ui,
                Rect::from_min_size(
                    pos2(hero.center().x - 120.0, hero.top() + 79.0),
                    vec2(240.0, 80.0),
                ),
            );
            ui.painter().text(
                pos2(hero.center().x, hero.top() + 181.0),
                Align2::CENTER_TOP,
                "Explore EA Play",
                shell::display(36.0),
                shell::TEXT,
            );
            let account_width = (width - 64.0).min(340.0).max(200.0);
            let account = Rect::from_center_size(
                pos2(hero.center().x, hero.top() + 284.0),
                vec2(account_width, 72.0),
            );
            ui.painter().rect_filled(
                account,
                8.0,
                Color32::from_rgba_unmultiplied(38, 38, 56, 235),
            );
            shell::account_avatar(
                app,
                ui,
                Rect::from_min_size(account.min + vec2(12.0, 12.0), vec2(48.0, 48.0)),
            );
            let name = if app.is_preview() {
                "Demo player"
            } else if app.user_id.is_empty() {
                "Not connected"
            } else {
                &app.user_name
            };
            text(
                ui,
                account.min + vec2(72.0, 12.0),
                name,
                shell::bold(16.0),
                shell::TEXT,
                account_width - 108.0,
            );
            text(
                ui,
                account.min + vec2(72.0, 40.0),
                if app.is_preview() {
                    "Sample membership · local preview"
                } else {
                    app.membership.description()
                },
                FontId::proportional(14.0),
                COPY,
                account_width - 108.0,
            );
            ui.advance_cursor_after_rect(hero);
            ui.label(egui::RichText::new("Check game trial availability").font(shell::bold(20.0)));
            ui.add_space(54.0);
            let trials: Vec<_> = catalog
                .products
                .iter()
                .filter(|product| {
                    let title = title(app, product).to_ascii_lowercase();
                    product.texture.is_some()
                        && !title.contains("bundle")
                        && (title.contains("fc™ 27")
                            || title.contains("college football 27")
                            || title.contains("madden nfl 27"))
                })
                .cloned()
                .take(3)
                .collect();
            trial_row(
                app,
                ui,
                if trials.is_empty() {
                    &catalog.products
                } else {
                    &trials
                },
            );
            ui.add_space(36.0);
            let browse_heading = ui.allocate_exact_size(vec2(width, 54.0), Sense::hover()).0;
            text(
                ui,
                browse_heading.min,
                "Browse EA games",
                shell::display(24.0),
                shell::TEXT,
                width - 140.0,
            );
            let view_all = ui
                .allocate_ui_at_rect(
                    Rect::from_min_size(
                        browse_heading.right_top() - vec2(108.0, 0.0),
                        vec2(108.0, 32.0),
                    ),
                    |ui| {
                        ui.add(
                            egui::Button::new(
                                egui::RichText::new("VIEW ALL").font(shell::bold(12.0)),
                            )
                            .fill(Color32::TRANSPARENT)
                            .rounding(4.0)
                            .min_size(vec2(108.0, 32.0)),
                        )
                        .clicked()
                    },
                )
                .inner;
            if view_all {
                app.page_view = PageType::Store;
            }
            ui.advance_cursor_after_rect(browse_heading);
            if catalog.products.is_empty() {
                ui.label(
                    egui::RichText::new("Public game artwork is not available locally.")
                        .color(COPY),
                );
            } else {
                let visible = catalog.products.len().min(5);
                product_grid(
                    app,
                    ui,
                    &catalog.products[..visible],
                    "ea-play-public-games",
                );
            }
            ui.add_space(18.0);
            ui.label(
                egui::RichText::new(
                    "These public products are not a verified EA Play inclusion list.",
                )
                .size(12.0)
                .color(COPY),
            );
            ui.add_space(80.0);
            comparison(app, ui);
            ui.add_space(48.0);
        });
}

#[cfg(test)]
mod catalog_tests {
    use super::{official_url, relative_image_path};
    use std::path::Path;

    #[test]
    fn official_links_accept_only_https_ea_sites_without_credentials() {
        for url in [
            "https://www.ea.com/ea-play",
            "https://ea.com/games/library",
            "https://store.ea.com/commerce/app/products/the-sims-4",
            "https://www.ea.com:443/games/library",
        ] {
            assert!(official_url(url), "{url}");
        }
        for url in [
            "http://www.ea.com/ea-play",
            "javascript:alert(1)",
            "https://www.ea.com.evil.example/game",
            "https://evil.example/ea.com",
            "https://www.ea.com@evil.example/",
            "https://user:password@www.ea.com/",
            "https://:password@www.ea.com/",
            "https://www.ea.com:8443/game",
            "file:///etc/passwd",
            "",
        ] {
            assert!(!official_url(url), "{url}");
        }
    }

    #[test]
    fn catalog_image_references_stay_relative_to_the_catalog_directory() {
        for value in ["cover.jpg", "images/cover.jpg", "images/nested/cover.png"] {
            let relative = relative_image_path(value).expect("valid image path");
            let root = Path::new("/local/catalog");
            assert!(root.join(relative).starts_with(root));
        }
        for value in [
            "",
            ".",
            "..",
            "../private.png",
            "images/../../private.png",
            "/etc/passwd",
            "images/../cover.jpg",
        ] {
            assert!(relative_image_path(value).is_none(), "{value}");
        }
    }
}

/// Known public URL only; callers never compose a guessed product slug.
pub fn product_url(ui: &Ui, slug: &str) -> String {
    catalog(ui)
        .products
        .into_iter()
        .find(|p| p.record.slug == slug)
        .map(|p| p.record.source_url)
        .unwrap_or_else(|| "https://www.ea.com/games/library".to_owned())
}
