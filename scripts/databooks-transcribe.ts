#!/usr/bin/env bun
/**
 * Boucle autonome de transcription des databooks : lot après lot, sans humain.
 *
 * Pour chaque lot restant sur le VPS :
 *   1. rapatrie `lot-NNN` s'il n'est pas déjà local ;
 *   2. lit les planches avec `aphrody ocr batch` (reprenable) ;
 *   3. renvoie le JSONL et le dépose via `depose-transcriptions.ts`.
 *
 * Conçue pour tourner des heures sans surveillance (cf. CLAUDE.md §0.1) :
 * chaque étape est reprenable, un lot en échec n'arrête pas les suivants, et
 * l'état vit sur le disque — relancer la commande continue où elle en était.
 *
 * Usage :
 *   bun scripts/databooks-transcribe.ts [--lots 1-29] [--travail <dir>]
 *                                       [--modele dots-ocr] [--simulation]
 *                                       [--garder-images] [--resident]
 *
 * ATTENTION : `--simulation` ne simule QUE le dépôt. Les planches sont bel et
 * bien lues par le GPU — soit environ une heure par lot. Pour vérifier la
 * chaîne rapidement, restreindre d'abord avec `--lots 1`.
 */

import { existsSync, mkdirSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { $ } from "bun";

const HOTE = process.env.DATABOOKS_SSH ?? "dbfr";
const LOTS_DISTANTS = "~/databooks-ocr";
const SITE_DISTANT = "~/shenron/apps/site";

interface Options {
	lots: number[];
	travail: string;
	modele: string;
	simulation: boolean;
	garderImages: boolean;
	resident: boolean;
	aphrody: string;
}

/** `--lots 3` ou `--lots 1-29` ou `--lots 2,5,7`. */
export function analyseLots(brut: string | undefined, total: number): number[] {
	if (!brut) return Array.from({ length: total }, (_, i) => i + 1);
	const out = new Set<number>();
	for (const morceau of brut.split(",")) {
		const plage = /^(\d+)-(\d+)$/.exec(morceau.trim());
		if (plage) {
			const [debut, fin] = [Number(plage[1]), Number(plage[2])];
			for (let n = Math.min(debut, fin); n <= Math.max(debut, fin); n++) out.add(n);
			continue;
		}
		const n = Number(morceau.trim());
		if (Number.isInteger(n) && n > 0) out.add(n);
	}
	return [...out].sort((a, b) => a - b);
}

/** `3` -> `lot-003`, la convention de `export-databooks-ocr.ts`. */
export function nomLot(n: number): string {
	return `lot-${String(n).padStart(3, "0")}`;
}

/** Combien de planches un dossier d'images contient. */
function compteImages(dir: string): number {
	if (!existsSync(dir)) return 0;
	return readdirSync(dir).filter((f) => /\.(jpe?g|png|webp|bmp)$/i.test(f)).length;
}

/** Combien de lignes un JSONL contient déjà. */
async function compteLignes(fichier: string): Promise<number> {
	if (!existsSync(fichier)) return 0;
	const texte = await Bun.file(fichier).text();
	return texte.split("\n").filter((l) => l.trim().length > 0).length;
}

function options(): Options {
	const args = Bun.argv.slice(2);
	const opt = (nom: string): string | undefined => {
		const i = args.indexOf(`--${nom}`);
		return i >= 0 ? args[i + 1] : undefined;
	};
	const defautTravail = join(process.env.TEMP ?? "/tmp", "databooks");
	return {
		lots: analyseLots(opt("lots"), 29),
		travail: opt("travail") ?? defautTravail,
		modele: opt("modele") ?? "dots-ocr",
		simulation: args.includes("--simulation"),
		garderImages: args.includes("--garder-images"),
		resident: args.includes("--resident"),
		aphrody: opt("aphrody") ?? process.env.APHRODY_BIN ?? "aphrody",
	};
}

/** Rapatrie un lot s'il n'est pas déjà complet localement. */
async function rapatrie(lot: string, o: Options): Promise<string> {
	const local = join(o.travail, lot);
	const images = join(local, "images");
	if (compteImages(images) > 0) {
		console.log(`  ${lot} : ${compteImages(images)} planche(s) déjà locales`);
		return local;
	}
	console.log(`  ${lot} : rapatriement…`);
	mkdirSync(o.travail, { recursive: true });
	await $`scp -q -r ${`${HOTE}:${LOTS_DISTANTS}/${lot}`} ${o.travail}`;
	console.log(`  ${lot} : ${compteImages(images)} planche(s) rapatriées`);
	return local;
}

/** Lit les planches. `--skip-done` rend l'appel reprenable. */
async function lis(lot: string, local: string, o: Options): Promise<string> {
	const jsonl = join(o.travail, `${lot}.jsonl`);
	const images = join(local, "images");
	const total = compteImages(images);
	const deja = await compteLignes(jsonl);
	if (deja >= total && total > 0) {
		console.log(`  ${lot} : déjà lu (${deja}/${total})`);
		return jsonl;
	}
	console.log(`  ${lot} : lecture de ${total - deja} planche(s) restantes…`);
	// Le modèle résident évite de recharger plusieurs gigaoctets par planche ;
	// le défaut reste le processus par planche, qui isole les pannes.
	const extra = o.resident ? ["--server"] : [];
	await $`${o.aphrody} ocr batch ${images} --model ${o.modele} --out ${jsonl} --skip-done ${extra}`;
	return jsonl;
}

/** Renvoie le JSONL et le dépose. */
async function depose(lot: string, jsonl: string, o: Options): Promise<void> {
	const lignes = await compteLignes(jsonl);
	if (lignes === 0) {
		console.log(`  ${lot} : rien à déposer`);
		return;
	}
	const distant = `${LOTS_DISTANTS}/${lot}-resultats.jsonl`;
	await $`scp -q ${jsonl} ${`${HOTE}:${distant}`}`;

	// Le jeton reste sur le VPS : il n'a aucune raison de transiter.
	const simulation = o.simulation ? "--simulation" : "";
	const commande =
		`export PATH="$HOME/.bun/bin:$PATH"; cd ${SITE_DISTANT} && ` +
		`export SHENRON_ADMIN_TOKEN=$(grep -m1 '^SHENRON_ADMIN_TOKEN=' .env | cut -d= -f2-) && ` +
		`bun scripts/depose-transcriptions.ts ${distant} ${simulation}`;
	const sortie = await $`ssh ${HOTE} ${commande}`.text();
	for (const ligne of sortie.trim().split("\n").slice(-4)) console.log(`    ${ligne}`);
}

async function main(): Promise<void> {
	const o = options();
	console.log(
		`transcription databooks — ${o.lots.length} lot(s), modèle ${o.modele}` +
			`${o.resident ? ", modèle résident" : ""}${o.simulation ? ", dépôt simulé" : ""}` +
			`\ntravail : ${o.travail}`,
	);
	if (o.simulation) {
		// « simulation » laisse croire que rien ne tourne ; la lecture, elle, a
		// bien lieu — et c'est le poste qui coûte des heures.
		console.log("note : --simulation ne simule que le dépôt, les planches sont lues normalement");
	}
	console.log("");

	let faits = 0;
	let echecs = 0;
	const debut = Date.now();

	for (const n of o.lots) {
		const lot = nomLot(n);
		console.log(`\n=== ${lot} (${faits + echecs + 1}/${o.lots.length}) ===`);
		try {
			const local = await rapatrie(lot, o);
			const jsonl = await lis(lot, local, o);
			await depose(lot, jsonl, o);

			// Les planches pèsent ~200 Mio par lot et ne servent plus une fois
			// lues ; le JSONL, lui, est la trace du travail et reste.
			if (!o.garderImages) {
				await $`rm -rf ${local}`.nothrow();
			}
			faits++;
		} catch (e) {
			// Un lot qui échoue ne doit pas arrêter les vingt-huit autres.
			echecs++;
			console.error(`  ${lot} ÉCHEC : ${e instanceof Error ? e.message : String(e)}`);
		}
		const ecoule = (Date.now() - debut) / 1000;
		console.log(`  cumul : ${faits} lot(s) faits, ${echecs} en échec, ${(ecoule / 60).toFixed(1)} min`);
	}

	console.log(`\n${faits}/${o.lots.length} lot(s) traités, ${echecs} en échec.`);
	if (echecs > 0) process.exit(1);
}

if (import.meta.main) await main();
