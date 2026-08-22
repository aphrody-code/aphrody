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
 *                                       [--garder-images] [--resident] [--force]
 *
 * ATTENTION : `--simulation` ne simule QUE le dépôt. Les planches sont bel et
 * bien lues par le GPU — soit environ une heure par lot. Pour vérifier la
 * chaîne rapidement, restreindre d'abord avec `--lots 1`.
 *
 * UNE SEULE INSTANCE : deux boucles se partagent le même GPU, saturent sa
 * mémoire et divisent le débit par huit — mesuré 10,9 s la planche seule contre
 * 90 s à deux, avec la VRAM à 95 % et le calcul retombé à 45 %. Le second
 * lancement est donc refusé par un verrou dans le dossier de travail, plutôt
 * que confié à la vigilance de qui lance la commande.
 */

import {
	existsSync,
	mkdirSync,
	readFileSync,
	readdirSync,
	rmSync,
	writeFileSync,
} from "node:fs";
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
	force: boolean;
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

/** Nombre de lots par défaut si le VPS ne répond pas. */
const LOTS_PAR_DEFAUT = 29;

/**
 * Combien de lots existent réellement sur le VPS.
 *
 * Le manifeste de chaque lot porte le total (`"lots": 29`). Le lire évite de
 * figer un compte qui changera au prochain export : un nombre codé en dur ferait
 * silencieusement sauter les lots ajoutés depuis.
 */
export function compteLotsDepuisManifeste(manifeste: string): number | null {
	try {
		const m = JSON.parse(manifeste) as { lots?: number };
		return Number.isInteger(m.lots) && (m.lots as number) > 0 ? (m.lots as number) : null;
	} catch {
		return null;
	}
}

async function detecteLots(): Promise<number> {
	const r = await $`ssh ${HOTE} cat ${`${LOTS_DISTANTS}/lot-001/manifeste.json`}`.nothrow().quiet();
	if (r.exitCode !== 0) return LOTS_PAR_DEFAUT;
	return compteLotsDepuisManifeste(r.stdout.toString()) ?? LOTS_PAR_DEFAUT;
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

function options(total: number): Options {
	const args = Bun.argv.slice(2);
	const opt = (nom: string): string | undefined => {
		const i = args.indexOf(`--${nom}`);
		return i >= 0 ? args[i + 1] : undefined;
	};
	const defautTravail = join(process.env.TEMP ?? "/tmp", "databooks");
	return {
		lots: analyseLots(opt("lots"), total),
		travail: opt("travail") ?? defautTravail,
		modele: opt("modele") ?? "dots-ocr",
		simulation: args.includes("--simulation"),
		garderImages: args.includes("--garder-images"),
		resident: args.includes("--resident"),
		force: args.includes("--force"),
		aphrody: opt("aphrody") ?? process.env.APHRODY_BIN ?? "aphrody",
	};
}

/** Ce qu'un fichier verrou contient. */
export interface Verrou {
	/** PID du processus qui tient le verrou. */
	pid: number;
	/** Date ISO du lancement, pour un message d'erreur lisible. */
	depuis: string;
}

/**
 * Le verrou est-il encore tenu par un processus vivant ?
 *
 * Un verrou survit à un `kill -9` et à une coupure de courant ; s'y fier
 * aveuglément condamnerait la boucle à refuser de redémarrer. On vérifie donc
 * le PID plutôt que la seule présence du fichier. `vivant` est injecté pour
 * que le test n'ait pas à créer de vrais processus.
 */
export function verrouTenu(
	contenu: string | null,
	vivant: (pid: number) => boolean,
): Verrou | null {
	if (!contenu) return null;
	let v: Partial<Verrou>;
	try {
		v = JSON.parse(contenu) as Partial<Verrou>;
	} catch {
		// Un verrou illisible est un verrou mort : un fichier tronqué par un
		// arrêt brutal ne doit pas bloquer la reprise.
		return null;
	}
	if (!Number.isInteger(v.pid) || (v.pid as number) <= 0) return null;
	if (!vivant(v.pid as number)) return null;
	return { pid: v.pid as number, depuis: typeof v.depuis === "string" ? v.depuis : "?" };
}

/** `process.kill(pid, 0)` : ne tue rien, lève si le PID n'existe pas. */
function pidVivant(pid: number): boolean {
	try {
		process.kill(pid, 0);
		return true;
	} catch (e) {
		// EPERM = le processus existe mais appartient à quelqu'un d'autre.
		return (e as NodeJS.ErrnoException).code === "EPERM";
	}
}

/**
 * Prend le verrou du dossier de travail, ou refuse le lancement.
 *
 * Renvoie la fonction qui le rend. Le verrou est aussi relâché sur SIGINT et
 * SIGTERM : une boucle interrompue au clavier ne doit pas laisser derrière elle
 * un fichier qui interdit la reprise.
 */
function prendVerrou(travail: string, force: boolean): () => void {
	const chemin = join(travail, ".verrou");
	const contenu = existsSync(chemin) ? readFileSync(chemin, "utf8") : null;
	const tenu = verrouTenu(contenu, pidVivant);
	if (tenu && !force) {
		console.error(
			`erreur : une boucle tourne déjà (pid ${tenu.pid}, depuis ${tenu.depuis}).\n` +
				"Deux boucles saturent la VRAM et divisent le débit par huit.\n" +
				`Arrêter l'autre, ou forcer avec --force si le pid est un faux positif.`,
		);
		process.exit(3);
	}
	if (tenu) {
		console.log(`note : --force, verrou du pid ${tenu.pid} ignoré`);
	}
	const mien: Verrou = { pid: process.pid, depuis: new Date().toISOString() };
	writeFileSync(chemin, JSON.stringify(mien));

	let rendu = false;
	const rends = (): void => {
		if (rendu) return;
		rendu = true;
		// Ne retirer que SON propre verrou : sous --force, un autre processus
		// peut avoir repris la main entre-temps.
		try {
			const actuel = existsSync(chemin) ? readFileSync(chemin, "utf8") : null;
			if (actuel && (JSON.parse(actuel) as Verrou).pid === process.pid) rmSync(chemin);
		} catch {
			/* un verrou déjà disparu n'est pas une erreur */
		}
	};
	for (const signal of ["SIGINT", "SIGTERM"] as const) {
		process.on(signal, () => {
			rends();
			process.exit(130);
		});
	}
	return rends;
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

/**
 * Vérifie le lot avant tout dépôt.
 *
 * Un dépôt est difficile à défaire : mieux vaut refuser un lot que d'écrire
 * quatre cents planches de sortie dégénérée dans un corpus public. `audit`
 * sort en code non nul sur un défaut bloquant, ce qui fait échouer le lot et
 * laisse la boucle passer au suivant.
 */
async function verifie(lot: string, jsonl: string, o: Options): Promise<void> {
	const r = await $`${o.aphrody} ocr audit ${jsonl}`.nothrow().quiet();
	const sortie = r.stdout.toString().trim().split("\n");
	const resume = sortie.find((l) => l.includes("with text")) ?? "";
	if (r.exitCode !== 0) {
		throw new Error(`audit refusé — ${resume || r.stderr.toString().trim().slice(0, 200)}`);
	}
	console.log(`  ${lot} : audit OK${resume ? ` — ${resume}` : ""}`);
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
	const total = await detecteLots();
	const o = options(total);
	// Avant tout travail : le dossier doit exister pour porter le verrou, et le
	// verrou doit être pris avant qu'un seul octet de VRAM soit réservé.
	mkdirSync(o.travail, { recursive: true });
	const rendsVerrou = prendVerrou(o.travail, o.force);
	let echecs = 0;
	try {
		echecs = await boucle(o);
	} finally {
		// Rendre le verrou avant de sortir : un `process.exit` dans la boucle
		// sauterait ce bloc et laisserait le prochain lancement bloqué.
		rendsVerrou();
	}
	if (echecs > 0) process.exit(1);
}

/** Le travail proprement dit, une fois le verrou acquis ; renvoie le nombre d'échecs. */
async function boucle(o: Options): Promise<number> {
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
			await verifie(lot, jsonl, o);
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
	return echecs;
}

if (import.meta.main) await main();
