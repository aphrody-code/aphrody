// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! Speech-balloon detection, so a comics page stops reading as blank.
//!
//! # Why this module exists
//!
//! A document vision model reads a printed page and returns nothing at all for
//! a comics page — the databook corpus has 1035 such plates. The received
//! explanation was that manga lettering is out of domain and needs a
//! specialised model. Measured on 2026-08-23, that is wrong: dots.ocr reads
//! `クッ!!` perfectly well **once the balloon is cropped out and enlarged**,
//! and returns nothing for the very same balloon left in its page.
//!
//! What it lacks is not vocabulary, it is framing. On a 1128x1600 plate a
//! balloon occupies 130x100 pixels — a handful of visual tokens inside an
//! image the model reads as a drawing. Tiling does not fix it either: the same
//! plate cut into twelve overlapping tiles and enlarged threefold, at six
//! times the cost, still returns nothing for that balloon. The model wants an
//! image whose *subject* is the text.
//!
//! So this module finds the balloons and hands each one over on its own.
//! Measured on twelve plates that the page-level pipeline reports as silent:
//! **44 balloons recovered across 9 of them**, carrying real dialogue —
//! `フリーザ様 たった今 カナッサ星を占領したという報告が入りました`.
//!
//! # What it does not do
//!
//! It does not segment balloons precisely, and it does not need to: a bounding
//! box the model can read is the whole requirement. It finds bright connected
//! regions, which is what a balloon is and also what a face, a sky and a pale
//! garment are. Those false positives cost a read each and return
//! [`PageText::None`](crate::PageText::None), so they are quiet — but they are
//! four fifths of the regions, and no cheap filter removes them. Ink coverage
//! was measured and rejected: balloons carrying text average 0.187 dark
//! pixels, regions carrying none average 0.195. The distributions overlap
//! completely, because the false positives are bright areas *inside drawings*,
//! full of linework.
//!
//! # The risk to guard against
//!
//! A region holding a *fragment* of a glyph — half a hand-drawn sound effect,
//! say — makes the model produce plausible wrong characters rather than
//! nothing: one orange katakana fragment came back as `禁 幸`. Balloon text
//! must therefore go through [`audit`](crate::audit)'s gibberish check before
//! it reaches a corpus. This module returns candidates, not verdicts.

use std::path::{Path, PathBuf};

use image::{DynamicImage, GenericImageView, imageops::FilterType};

/// A rectangle on the page, in pixels, that may hold text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bulle {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width in pixels.
    pub largeur: u32,
    /// Height in pixels.
    pub hauteur: u32,
    /// Bright pixels the region actually contains, its area proper.
    pub aire: u32,
}

impl Bulle {
    /// Right edge, exclusive.
    #[must_use]
    pub const fn droite(&self) -> u32 {
        self.x + self.largeur
    }

    /// Bottom edge, exclusive.
    #[must_use]
    pub const fn bas(&self) -> u32 {
        self.y + self.hauteur
    }
}

/// Thresholds for [`detecte`], all measured rather than guessed.
#[derive(Debug, Clone, Copy)]
pub struct ReglagesBulles {
    /// Luma above which a pixel counts as balloon-bright.
    ///
    /// 200 of 255. Balloon interiors are paper-white even in a colour comic;
    /// lowering this floods the mask with pale sky and skin.
    pub seuil_clair: u8,

    /// Smallest region kept, as a fraction of the page.
    ///
    /// A balloon carrying one syllable is about 1200 pixels on a 1.8 megapixel
    /// plate. Expressed as a fraction so a rescanned corpus behaves the same.
    pub aire_min_relative: f32,

    /// Largest region kept, as a fraction of the page. Above this it is the
    /// background, not a balloon.
    pub aire_max_relative: f32,

    /// Smallest side kept, in pixels. Below it there is no room for a glyph.
    pub cote_min: u32,

    /// Width-to-height bounds. Outside them the region is a border or a gutter,
    /// not a balloon.
    pub rapport_min: f32,
    /// Upper bound of the width-to-height ratio.
    pub rapport_max: f32,

    /// How much of its bounding box the region must fill.
    ///
    /// A balloon is convex enough to fill most of its box; an L-shaped sliver
    /// of background is not.
    pub remplissage_min: f32,

    /// Most regions handed back for one page.
    ///
    /// Bounds the cost: each region is one model read. Regions are returned
    /// largest first, so the cut falls on the least likely candidates.
    pub maximum: usize,
}

impl Default for ReglagesBulles {
    fn default() -> Self {
        Self {
            seuil_clair: 200,
            aire_min_relative: 1200.0 / 1_804_800.0,
            aire_max_relative: 0.25,
            cote_min: 25,
            rapport_min: 0.15,
            rapport_max: 7.0,
            remplissage_min: 0.45,
            maximum: 30,
        }
    }
}

/// Find the regions of `image` that may hold lettering.
///
/// Returned largest first, so truncating the list drops the least likely
/// candidates. Use [`ordonne_lecture`] to put them back in reading order.
#[must_use]
pub fn detecte(image: &DynamicImage, reglages: &ReglagesBulles) -> Vec<Bulle> {
    let gris = image.to_luma8();
    let (largeur, hauteur) = gris.dimensions();
    if largeur == 0 || hauteur == 0 {
        return Vec::new();
    }

    let etiquettes = etiquette_composantes(&gris, reglages.seuil_clair);
    let aire_page = f64::from(largeur) * f64::from(hauteur);
    let aire_min = aire_page * f64::from(reglages.aire_min_relative);
    let aire_max = aire_page * f64::from(reglages.aire_max_relative);

    let mut bulles: Vec<Bulle> = etiquettes
        .into_iter()
        .filter(|b| retenue(b, reglages, aire_min, aire_max))
        .collect();

    // Les plus grandes d'abord : c'est sur elles qu'on veut dépenser le budget
    // de lecture si `maximum` tranche.
    bulles.sort_unstable_by(|a, b| b.aire.cmp(&a.aire));
    bulles.truncate(reglages.maximum);
    bulles
}

/// Whether a raw connected component passes every shape test.
fn retenue(b: &Bulle, reglages: &ReglagesBulles, aire_min: f64, aire_max: f64) -> bool {
    let aire = f64::from(b.aire);
    if aire < aire_min || aire > aire_max {
        return false;
    }
    if b.largeur < reglages.cote_min || b.hauteur < reglages.cote_min {
        return false;
    }
    let rapport = f64::from(b.largeur) / f64::from(b.hauteur);
    if rapport <= f64::from(reglages.rapport_min) || rapport >= f64::from(reglages.rapport_max) {
        return false;
    }
    let boite = f64::from(b.largeur) * f64::from(b.hauteur);
    aire / boite >= f64::from(reglages.remplissage_min)
}

/// Put balloons in Japanese comics reading order: top to bottom, and right to
/// left within a band.
///
/// # Why bands are assigned before sorting, not inside the comparison
///
/// The obvious version — "if these two overlap vertically, compare by x, else
/// by y" — is not a total order, and Rust's sort detects it and panics. It is
/// not transitive: A can overlap B and B overlap C without A overlapping C, so
/// the same three balloons compare inconsistently depending on the pairs the
/// sort happens to pick. It survived a three-balloon test with well-separated
/// rows and died on the first real plate.
///
/// So banding is decided once, globally: sweep the balloons top-down, and open
/// a new band whenever one no longer overlaps the band being built by at least
/// half its own height. Sorting then compares `(band, -x)`, which is a total
/// order by construction.
pub fn ordonne_lecture(bulles: &mut [Bulle]) {
    if bulles.len() < 2 {
        return;
    }
    let mut ordre: Vec<usize> = (0..bulles.len()).collect();
    ordre.sort_unstable_by_key(|&i| (bulles[i].y, bulles[i].x));

    // Bande courante : l'intervalle vertical couvert par ses membres.
    let mut bande = vec![0_usize; bulles.len()];
    let mut numero = 0_usize;
    let (mut haut, mut bas) = (bulles[ordre[0]].y, bulles[ordre[0]].bas());
    for &i in &ordre {
        let b = bulles[i];
        let recouvrement = bas.min(b.bas()).saturating_sub(haut.max(b.y));
        if recouvrement * 2 <= b.hauteur {
            numero += 1;
            haut = b.y;
            bas = b.bas();
        } else {
            // La bande s'étire : une réplique plus haute que ses voisines ne
            // doit pas fermer la ligne derrière elle.
            haut = haut.min(b.y);
            bas = bas.max(b.bas());
        }
        bande[i] = numero;
    }

    // Trier des indices plutôt que les bulles : la clé a besoin de la bande,
    // qui est indexée par position. `Reverse` sur x parce que le japonais se
    // lit de droite à gauche ; y départage deux bulles superposées.
    let mut ordre_final: Vec<usize> = (0..bulles.len()).collect();
    ordre_final
        .sort_unstable_by_key(|&i| (bande[i], std::cmp::Reverse(bulles[i].x), bulles[i].y));
    let trie: Vec<Bulle> = ordre_final.into_iter().map(|i| bulles[i]).collect();
    bulles.copy_from_slice(&trie);
}

/// Crop `bulle` out of `image`, with a margin, enlarged for the model.
///
/// The margin keeps the balloon's outline in frame; without it the first and
/// last glyph sit flush against the edge and get clipped. The enlargement is
/// what makes the read work at all — a raw crop of a small balloon still
/// returns nothing, the same crop at three times the size reads `クッ!!`.
#[must_use]
pub fn recadre(image: &DynamicImage, bulle: &Bulle, cote_cible: u32) -> DynamicImage {
    let (largeur, hauteur) = image.dimensions();
    let marge_x = (bulle.largeur / 12).max(6);
    let marge_y = (bulle.hauteur / 12).max(6);
    let x0 = bulle.x.saturating_sub(marge_x);
    let y0 = bulle.y.saturating_sub(marge_y);
    let x1 = bulle.droite().saturating_add(marge_x).min(largeur);
    let y1 = bulle.bas().saturating_add(marge_y).min(hauteur);
    let coupe = image.crop_imm(x0, y0, x1 - x0, y1 - y0);

    let facteur = facteur_agrandissement(coupe.width().max(coupe.height()), cote_cible);
    if facteur == 1 {
        return coupe;
    }
    coupe.resize(
        coupe.width() * facteur,
        coupe.height() * facteur,
        // Lanczos : le texte de bulle est fin, un rééchantillonnage plus
        // brutal l'épaissit jusqu'à coller les traits d'un kana.
        FilterType::Lanczos3,
    )
}

/// Crop every balloon of `planche` into `dossier`, in reading order.
///
/// Returns the crops as written, named `<plate>-bNN.jpg` so that a file's
/// number is its rank in the dialogue — which is what lets a caller reassemble
/// the page, and what lets a person check a surprising line against a picture.
///
/// The pixels stay in this crate. A caller that wanted to do this itself would
/// have to depend on an image decoder to re-implement the framing rules that
/// make the read work at all, and those rules were measured here.
///
/// # Errors
///
/// When `planche` cannot be decoded, or a crop cannot be written to `dossier`.
/// The directory must already exist; whether it is temporary or kept is the
/// caller's decision, not this function's.
pub fn decoupe_planche(
    planche: &Path,
    dossier: &Path,
    reglages: &ReglagesBulles,
    cible: u32,
) -> crate::Result<Vec<PathBuf>> {
    let image = image::open(planche).map_err(|e| crate::OcrError::Io {
        path: planche.to_path_buf(),
        source: std::io::Error::other(e),
    })?;

    let mut trouvees = detecte(&image, reglages);
    if trouvees.is_empty() {
        return Ok(Vec::new());
    }
    ordonne_lecture(&mut trouvees);

    let tige = planche.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let mut chemins = Vec::with_capacity(trouvees.len());
    for (i, bulle) in trouvees.iter().enumerate() {
        let chemin = dossier.join(format!("{tige}-b{i:02}.jpg"));
        recadre(&image, bulle, cible).save(&chemin).map_err(|e| crate::OcrError::Io {
            path: chemin.clone(),
            source: std::io::Error::other(e),
        })?;
        chemins.push(chemin);
    }
    Ok(chemins)
}

/// Integer enlargement bringing `cote` towards `cible`, clamped to 2..=5.
///
/// Never 1: an un-enlarged crop is the case that measured as unreadable. Never
/// past 5, where the image costs visual tokens without carrying more detail —
/// the pixels are interpolated, not recovered.
#[must_use]
pub fn facteur_agrandissement(cote: u32, cible: u32) -> u32 {
    if cote == 0 {
        return 2;
    }
    (cible / cote).clamp(2, 5)
}

/// One connected component's bounding box and area, before filtering.
type Composante = Bulle;

/// Label bright connected components, four-connectivity, union-find.
///
/// Two passes rather than a flood fill per pixel: a plate is two megapixels and
/// balloons are large, so a recursive fill would run deep enough to matter.
fn etiquette_composantes(gris: &image::GrayImage, seuil: u8) -> Vec<Composante> {
    let (largeur, hauteur) = gris.dimensions();
    let n = (largeur as usize) * (hauteur as usize);
    let mut parent: Vec<u32> = Vec::new();
    let mut etiquettes = vec![0_u32; n];
    // Étiquettes numérotées à partir de 1, pour que 0 puisse vouloir dire
    // « pas d'étiquette ».
    let mut prochaine: u32 = 0;

    // Première passe : étiqueter, et noter les équivalences entre le voisin
    // du haut et celui de gauche.
    for y in 0..hauteur {
        for x in 0..largeur {
            if gris.get_pixel(x, y).0[0] <= seuil {
                continue;
            }
            let i = (y as usize) * (largeur as usize) + (x as usize);
            let gauche = if x > 0 { etiquettes[i - 1] } else { 0 };
            let haut = if y > 0 {
                etiquettes[i - (largeur as usize)]
            } else {
                0
            };
            etiquettes[i] = match (gauche, haut) {
                (0, 0) => {
                    parent.push(prochaine);
                    prochaine += 1;
                    prochaine
                }
                (g, 0) => g,
                (0, h) => h,
                (g, h) => {
                    unir(&mut parent, g, h);
                    g.min(h)
                }
            };
        }
    }

    // Seconde passe : résoudre chaque étiquette vers sa racine et accumuler.
    let mut boites: std::collections::HashMap<u32, Composante> = std::collections::HashMap::new();
    for y in 0..hauteur {
        for x in 0..largeur {
            let i = (y as usize) * (largeur as usize) + (x as usize);
            let e = etiquettes[i];
            if e == 0 {
                continue;
            }
            let racine = trouve(&mut parent, e);
            let entree = boites.entry(racine).or_insert(Bulle {
                x,
                y,
                largeur: 1,
                hauteur: 1,
                aire: 0,
            });
            let x0 = entree.x.min(x);
            let y0 = entree.y.min(y);
            let x1 = entree.droite().max(x + 1);
            let y1 = entree.bas().max(y + 1);
            entree.x = x0;
            entree.y = y0;
            entree.largeur = x1 - x0;
            entree.hauteur = y1 - y0;
            entree.aire += 1;
        }
    }
    boites.into_values().collect()
}

/// Union-find root of label `e`, with path halving.
///
/// Labels are 1-based so that zero can mean "no label"; `parent` is indexed
/// from zero, hence the shifts.
fn trouve(parent: &mut [u32], e: u32) -> u32 {
    let mut i = (e - 1) as usize;
    while parent[i] as usize != i {
        parent[i] = parent[parent[i] as usize];
        i = parent[i] as usize;
    }
    u32::try_from(i).expect("label index fits in u32") + 1
}

/// Merge the sets of two labels.
fn unir(parent: &mut [u32], a: u32, b: u32) {
    let ra = trouve(parent, a) - 1;
    let rb = trouve(parent, b) - 1;
    if ra == rb {
        return;
    }
    let (petit, grand) = if ra < rb { (ra, rb) } else { (rb, ra) };
    parent[grand as usize] = petit;
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    /// A dark page with bright rectangles painted on it.
    fn page(rectangles: &[(u32, u32, u32, u32)]) -> DynamicImage {
        let mut im = RgbImage::from_pixel(1128, 1600, Rgb([10, 10, 10]));
        for &(x, y, w, h) in rectangles {
            for yy in y..y + h {
                for xx in x..x + w {
                    im.put_pixel(xx, yy, Rgb([250, 250, 250]));
                }
            }
        }
        DynamicImage::ImageRgb8(im)
    }

    #[test]
    fn trouve_un_rectangle_de_la_taille_d_une_bulle() {
        let im = page(&[(100, 200, 130, 100)]);
        let bulles = detecte(&im, &ReglagesBulles::default());
        assert_eq!(bulles.len(), 1);
        let b = bulles[0];
        assert_eq!((b.x, b.y, b.largeur, b.hauteur), (100, 200, 130, 100));
    }

    #[test]
    fn ignore_le_bruit_et_le_fond() {
        // Un grain de 10x10 est sous l'aire minimale ; un aplat de 900x900
        // dépasse le quart de page et se lit comme fond.
        let im = page(&[(10, 10, 10, 10), (100, 100, 900, 900)]);
        assert!(detecte(&im, &ReglagesBulles::default()).is_empty());
    }

    #[test]
    fn ignore_un_lisere() {
        // 400x30 : le rapport dépasse la borne, c'est une gouttière.
        let im = page(&[(50, 50, 400, 30)]);
        assert!(detecte(&im, &ReglagesBulles::default()).is_empty());
    }

    #[test]
    fn rend_les_plus_grandes_d_abord_et_borne_le_nombre() {
        let mut rects = Vec::new();
        for i in 0..40_u32 {
            let cote = 60 + i;
            rects.push((10 + (i % 10) * 110, 10 + (i / 10) * 150, cote, cote));
        }
        let im = page(&rects);
        let reglages = ReglagesBulles::default();
        let bulles = detecte(&im, &reglages);
        assert_eq!(bulles.len(), reglages.maximum);
        assert!(
            bulles.windows(2).all(|p| p[0].aire >= p[1].aire),
            "les régions doivent sortir de la plus grande à la plus petite"
        );
    }

    #[test]
    fn ordre_de_lecture_droite_a_gauche_puis_haut_en_bas() {
        // Deux bulles sur la même bande, une troisième dessous.
        let mut bulles = vec![
            Bulle { x: 100, y: 100, largeur: 100, hauteur: 100, aire: 10_000 },
            Bulle { x: 800, y: 110, largeur: 100, hauteur: 100, aire: 10_000 },
            Bulle { x: 400, y: 900, largeur: 100, hauteur: 100, aire: 10_000 },
        ];
        ordonne_lecture(&mut bulles);
        assert_eq!(bulles[0].x, 800, "la bulle de droite se lit en premier");
        assert_eq!(bulles[1].x, 100);
        assert_eq!(bulles[2].y, 900, "la bande du dessous vient après");
    }

    #[test]
    fn une_chaine_de_bulles_en_escalier_ne_casse_pas_le_tri() {
        // Le cas qui a fait paniquer la premiere version : chaque bulle
        // chevauche sa voisine sans chevaucher la suivante, donc « meme
        // bande » decide dans le comparateur n'est pas transitif. Rust
        // detecte l'ordre partiel et panique — sur une planche reelle, pas
        // sur trois bulles bien separees.
        let mut bulles: Vec<Bulle> = (0..12_u32)
            .map(|i| Bulle {
                x: 900 - i * 70,
                y: 100 + i * 60,
                largeur: 100,
                hauteur: 100,
                aire: 10_000,
            })
            .collect();
        ordonne_lecture(&mut bulles);
        assert_eq!(bulles.len(), 12, "aucune bulle perdue par le tri");
        // Et l'ordre reste deterministe : deux appels donnent le meme.
        let premier = bulles.clone();
        ordonne_lecture(&mut bulles);
        assert_eq!(premier, bulles, "le tri doit etre idempotent");
    }

    #[test]
    fn une_bulle_seule_traverse_le_tri() {
        let mut une = vec![Bulle { x: 10, y: 10, largeur: 50, hauteur: 50, aire: 2500 }];
        ordonne_lecture(&mut une);
        assert_eq!(une.len(), 1);
        ordonne_lecture(&mut []);
    }

    #[test]
    fn le_recadrage_agrandit_et_garde_une_marge() {
        let im = page(&[(100, 200, 130, 100)]);
        let b = Bulle { x: 100, y: 200, largeur: 130, hauteur: 100, aire: 13_000 };
        let coupe = recadre(&im, &b, 900);
        // 130 + 2*10 de marge = 150, agrandi ×6 borné à ×5.
        assert_eq!(coupe.width(), 150 * 5);
        assert_eq!(coupe.height(), (100 + 2 * 8) * 5);
    }

    #[test]
    fn le_recadrage_ne_deborde_pas_du_bord() {
        let im = page(&[(0, 0, 130, 100)]);
        let b = Bulle { x: 0, y: 0, largeur: 130, hauteur: 100, aire: 13_000 };
        let coupe = recadre(&im, &b, 900);
        assert!(coupe.width() > 0 && coupe.height() > 0);
    }

    #[test]
    fn l_agrandissement_ne_descend_jamais_a_un() {
        assert_eq!(facteur_agrandissement(1000, 900), 2, "jamais 1 : c'est le cas illisible");
        assert_eq!(facteur_agrandissement(300, 900), 3);
        assert_eq!(facteur_agrandissement(10, 900), 5, "borné à 5 : au-delà les pixels sont interpolés");
    }

    #[test]
    fn le_decoupage_ecrit_un_fichier_par_bulle_dans_l_ordre_de_lecture() {
        let dossier = tempfile::tempdir().expect("dossier temporaire");
        let planche = dossier.path().join("312-0014.jpg");
        // Deux bulles sur la meme bande : la droite se lit en premier, donc
        // c'est elle qui doit porter le numero 00.
        page(&[(700, 100, 130, 100), (100, 110, 130, 100)])
            .save(&planche)
            .expect("planche de test");

        let sortie = dossier.path().join("crops");
        std::fs::create_dir_all(&sortie).expect("dossier de sortie");
        let crops = decoupe_planche(&planche, &sortie, &ReglagesBulles::default(), 900)
            .expect("decoupage");

        assert_eq!(crops.len(), 2);
        assert!(crops[0].ends_with("312-0014-b00.jpg"));
        assert!(crops[1].ends_with("312-0014-b01.jpg"));
        assert!(crops.iter().all(|c| c.exists()), "chaque bulle doit etre sur le disque");
    }

    #[test]
    fn une_planche_sans_bulle_ne_rend_aucun_fichier() {
        let dossier = tempfile::tempdir().expect("dossier temporaire");
        let planche = dossier.path().join("vide.jpg");
        page(&[]).save(&planche).expect("planche de test");
        let crops = decoupe_planche(&planche, dossier.path(), &ReglagesBulles::default(), 900)
            .expect("decoupage");
        assert!(crops.is_empty());
    }

    #[test]
    fn deux_regions_en_forme_de_u_fusionnent_en_une_composante() {
        // Un U : deux montants réunis par une base. La passe d'union doit les
        // rendre comme UNE composante, pas trois.
        let im = page(&[(100, 100, 40, 200), (300, 100, 40, 200), (100, 260, 240, 40)]);
        let bulles = detecte(&im, &ReglagesBulles::default());
        assert_eq!(bulles.len(), 1, "les trois barres se touchent : une seule région");
        // Il remplit 53 % de sa boîte, juste au-dessus du seuil : c'est
        // volontairement un cas limite, pour que le filtre de remplissage
        // bouge si quelqu'un le retouche.
        assert_eq!(bulles[0].largeur, 240);
    }
}
