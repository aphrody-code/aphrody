#!/usr/bin/env bun
/**
 * État d'avancement de la transcription, lu depuis l'API publique du site.
 *
 * L'audit se faisait jusqu'ici en SSH + psql sur le VPS : il fallait un accès
 * à la machine et aux identifiants de la base pour répondre à « où en est-on ».
 * Or la seule réponse qui compte est celle que voit un lecteur du site, et
 * elle est publique. Ce script la lit donc par HTTP, sans SSH, sans jeton, et
 * peut tourner depuis n'importe quelle machine — y compris pendant que la
 * boucle de transcription occupe le GPU.
 *
 * Ce que le site renvoie et ce qu'il ne renvoie pas : `/api/databooks/{id}`
 * livre chaque planche avec son texte, jamais de compteur. `pagesTranscrites`
 * et `pageCount` que l'on voit via le serveur MCP sont calculés par celui-ci,
 * pas par le site. On les recalcule donc ici de la même façon : une planche
 * est transcrite si son `text` existe et n'est pas vide une fois détouré.
 *
 * Usage :
 *   bun scripts/databooks-avancement.ts [--json] [--detail] [--vides]
 *                                       [--parallele 8] [--base <url>]
 *
 *   --detail   une ligne par ouvrage plutôt qu'un total par catégorie
 *   --vides    ne lister que les ouvrages sans la moindre planche transcrite
 */

const BASE = "https://dragonballfr.com";

export interface Planche {
    number: number;
    image?: string | null;
    text?: string | null;
}

export interface Ouvrage {
    id: number;
    title: string;
    category?: string | null;
    kind?: string | null;
    pages?: Planche[] | null;
}

export interface Avancement {
    id: number;
    titre: string;
    categorie: string;
    planches: number;
    transcrites: number;
    sansScan: number;
}

/**
 * Une planche sans scan ET sans texte ne sera jamais lue : il n'y a rien à lire.
 *
 * La base porte des lignes de planche vides — `{"number":5,"image":null,
 * "text":null}` — réservées pour un scan qui n'est jamais arrivé. Mesuré le
 * 2026-08-22 : 262 planches, dont 228 pour le seul Daizenshuu 1. Les compter
 * comme du travail en retard fait mentir l'avancement de deux points et promet
 * une fin qui ne peut pas venir.
 *
 * L'absence d'image ne suffit pas comme critère : 236 planches portent du texte
 * venu d'une autre source sans avoir de scan en base. Les retirer du
 * dénominateur tout en comptant leur texte ferait passer des ouvrages
 * au-dessus de 100 %.
 */
export function sansScan(planche: Planche): boolean {
    const image = typeof planche.image === "string" && planche.image.trim().length > 0;
    return !image && !estTranscrite(planche);
}

/**
 * Une planche compte comme transcrite si elle porte du texte non vide.
 *
 * Le contrat de dépôt distingue trois états et les confond volontiers : `null`
 * efface une transcription, `""` en enregistre une vide, et une chaîne d'espaces
 * vient d'une planche muette qu'on a quand même déposée. Les trois signifient
 * « rien à lire » pour un lecteur, donc rien pour ce compteur.
 */
export function estTranscrite(planche: Planche): boolean {
    return typeof planche.text === "string" && planche.text.trim().length > 0;
}

export function avancementOuvrage(o: Ouvrage): Avancement {
    const pages = o.pages ?? [];
    return {
        id: o.id,
        titre: o.title,
        categorie: o.category ?? "(sans catégorie)",
        // `planches` est le lisible, pas le total : une planche sans scan sort
        // du dénominateur, sinon l'avancement ne peut jamais atteindre 100 %.
        planches: pages.length - pages.filter(sansScan).length,
        transcrites: pages.filter(estTranscrite).length,
        sansScan: pages.filter(sansScan).length,
    };
}

export interface Total {
    categorie: string;
    ouvrages: number;
    planches: number;
    transcrites: number;
    sansScan: number;
}

/** Regroupe par catégorie, la plus fournie en tête. */
export function totalise(lignes: Avancement[]): Total[] {
    const par = new Map<string, Total>();
    for (const l of lignes) {
        const t = par.get(l.categorie) ?? {
            categorie: l.categorie,
            ouvrages: 0,
            planches: 0,
            transcrites: 0,
            sansScan: 0,
        };
        t.ouvrages += 1;
        t.planches += l.planches;
        t.transcrites += l.transcrites;
        t.sansScan += l.sansScan;
        par.set(l.categorie, t);
    }
    return [...par.values()].toSorted((a, b) => b.planches - a.planches);
}

export function pourcent(fait: number, total: number): string {
    if (total === 0) return "  -  ";
    return `${((fait / total) * 100).toFixed(1).padStart(5)}%`;
}

/**
 * Exécute `travail` sur chaque entrée, `largeur` en vol à la fois.
 *
 * Le site est public et n'est pas à nous : lancer 318 requêtes d'un coup
 * fonctionnerait sans doute, et serait quand même une impolitesse gratuite
 * pour un audit qui n'est pressé par rien.
 */
export async function parLots<T, R>(
    entrees: T[],
    largeur: number,
    travail: (e: T) => Promise<R>,
): Promise<R[]> {
    const out = Array.from({ length: entrees.length }) as R[];
    let suivant = 0;
    const ouvrier = async (): Promise<void> => {
        for (;;) {
            const i = suivant++;
            if (i >= entrees.length) return;
            // L'attente séquentielle EST la limite de parallélisme : chaque ouvrier
            // prend la tâche suivante quand il a fini la sienne.
            // oxlint-disable-next-line no-await-in-loop
            out[i] = await travail(entrees[i] as T);
        }
    };
    await Promise.all(Array.from({ length: Math.min(largeur, entrees.length) }, ouvrier));
    return out;
}

async function json(url: string): Promise<unknown> {
    const r = await fetch(url, { headers: { accept: "application/json" } });
    if (!r.ok) throw new Error(`${url} → HTTP ${r.status}`);
    return r.json();
}

/** L'API pagine ; on la déroule jusqu'au bout plutôt que de parier sur un plafond. */
async function listeOuvrages(base: string): Promise<Ouvrage[]> {
    const tout: Ouvrage[] = [];
    const pas = 100;
    for (let offset = 0; ; offset += pas) {
        // La page suivante dépend du nombre d'éléments reçus : une pagination ne
        // peut pas partir en parallèle.
        // oxlint-disable-next-line no-await-in-loop
        const page = (await json(`${base}/api/databooks?limit=${pas}&offset=${offset}`)) as {
            items?: Ouvrage[];
            total?: number;
        };
        const items = page.items ?? [];
        tout.push(...items);
        if (items.length < pas) return tout;
        // Un `total` incohérent ne doit pas boucler à l'infini.
        if (typeof page.total === "number" && tout.length >= page.total) return tout;
    }
}

interface Options {
    json: boolean;
    detail: boolean;
    vides: boolean;
    parallele: number;
    base: string;
}

function options(argv: string[]): Options {
    const valeur = (nom: string): string | undefined => {
        const i = argv.indexOf(nom);
        return i >= 0 ? argv[i + 1] : undefined;
    };
    const largeur = Number.parseInt(valeur("--parallele") ?? "8", 10);
    return {
        json: argv.includes("--json"),
        detail: argv.includes("--detail"),
        vides: argv.includes("--vides"),
        parallele: Number.isFinite(largeur) && largeur > 0 ? largeur : 8,
        base: valeur("--base") ?? BASE,
    };
}

async function main(): Promise<void> {
    const o = options(process.argv.slice(2));
    const catalogue = await listeOuvrages(o.base);

    // La liste porte déjà les planches et leur texte : inutile de redemander
    // chaque fiche. Mais on ne le suppose pas — un ouvrage renvoyé sans `pages`
    // est rattrapé par sa fiche détaillée, sinon il compterait pour zéro.
    const lignes = await parLots(catalogue, o.parallele, async (o2) => {
        if (o2.pages) return avancementOuvrage(o2);
        const fiche = (await json(`${o.base}/api/databooks/${o2.id}`)) as Ouvrage;
        return avancementOuvrage({ ...o2, ...fiche });
    });

    const planches = lignes.reduce((s, l) => s + l.planches, 0);
    const transcrites = lignes.reduce((s, l) => s + l.transcrites, 0);
    const orphelines = lignes.reduce((s, l) => s + l.sansScan, 0);

    if (o.json) {
        console.log(
            JSON.stringify(
                { ouvrages: lignes.length, planches, transcrites, sansScan: orphelines, lignes },
                null,
                2,
            ),
        );
        return;
    }

    if (o.vides) {
        const rien = lignes
            .filter((l) => l.transcrites === 0 && l.planches > 0)
            .toSorted((a, b) => b.planches - a.planches);
        console.log(`${rien.length} ouvrage(s) sans aucune planche transcrite :`);
        for (const l of rien) {
            console.log(
                `  #${String(l.id).padStart(3)}  ${String(l.planches).padStart(4)} pl.  ${l.titre}`,
            );
        }
    } else if (o.detail) {
        for (const l of lignes.toSorted((a, b) => a.id - b.id)) {
            console.log(
                `#${String(l.id).padStart(3)}  ${String(l.transcrites).padStart(4)}/${String(l.planches).padEnd(4)} ${pourcent(l.transcrites, l.planches)}  ${l.titre}`,
            );
        }
    } else {
        for (const t of totalise(lignes)) {
            console.log(
                `${t.categorie.padEnd(24)} ${String(t.ouvrages).padStart(4)} ouvr.  ${String(t.transcrites).padStart(5)}/${String(t.planches).padEnd(5)} ${pourcent(t.transcrites, t.planches)}`,
            );
        }
    }

    console.log(
        `\n${lignes.length} ouvrage(s), ${transcrites}/${planches} planche(s) transcrite(s) — ` +
            pourcent(transcrites, planches) +
            (orphelines > 0 ? ` (${orphelines} sans scan, hors compte)` : ""),
    );
}

if (import.meta.main) await main();
