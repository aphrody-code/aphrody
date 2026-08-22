// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

import { describe, expect, test } from "bun:test";

import { analyseLots, compteLotsDepuisManifeste, nomLot, verrouTenu } from "./databooks-transcribe";

describe("analyseLots", () => {
	test("sans argument, tous les lots", () => {
		expect(analyseLots(undefined, 4)).toEqual([1, 2, 3, 4]);
	});

	test("un seul lot", () => {
		expect(analyseLots("3", 29)).toEqual([3]);
	});

	test("une plage, dans les deux sens", () => {
		expect(analyseLots("2-5", 29)).toEqual([2, 3, 4, 5]);
		expect(analyseLots("5-2", 29)).toEqual([2, 3, 4, 5]);
	});

	test("une liste, dédoublonnée et triée", () => {
		expect(analyseLots("7,2,7,1-2", 29)).toEqual([1, 2, 7]);
	});
});

describe("nomLot", () => {
	test("rembourre sur trois chiffres", () => {
		expect(nomLot(3)).toBe("lot-003");
		expect(nomLot(29)).toBe("lot-029");
	});
});

describe("compteLotsDepuisManifeste", () => {
	test("lit le total", () => {
		expect(compteLotsDepuisManifeste('{"lots":29}')).toBe(29);
	});

	test("refuse un manifeste illisible ou absurde", () => {
		expect(compteLotsDepuisManifeste("pas du json")).toBeNull();
		expect(compteLotsDepuisManifeste('{"lots":0}')).toBeNull();
		expect(compteLotsDepuisManifeste("{}")).toBeNull();
	});
});

describe("verrouTenu", () => {
	const vivant = () => true;
	const mort = () => false;

	test("pas de fichier, pas de verrou", () => {
		expect(verrouTenu(null, vivant)).toBeNull();
	});

	test("un pid vivant tient le verrou", () => {
		const v = verrouTenu('{"pid":4242,"depuis":"2026-08-22T05:00:00.000Z"}', vivant);
		expect(v).toEqual({ pid: 4242, depuis: "2026-08-22T05:00:00.000Z" });
	});

	test("un pid mort ne tient rien — sinon un kill -9 condamnerait la reprise", () => {
		expect(verrouTenu('{"pid":4242}', mort)).toBeNull();
	});

	test("un verrou tronqué par un arrêt brutal est traité comme mort", () => {
		expect(verrouTenu('{"pid":42', vivant)).toBeNull();
	});

	test("un pid absurde est ignoré plutôt que cru", () => {
		expect(verrouTenu('{"pid":0}', vivant)).toBeNull();
		expect(verrouTenu('{"pid":-1}', vivant)).toBeNull();
		expect(verrouTenu('{"pid":"beaucoup"}', vivant)).toBeNull();
	});

	test("une date manquante ne casse pas le message d'erreur", () => {
		expect(verrouTenu('{"pid":7}', vivant)).toEqual({ pid: 7, depuis: "?" });
	});
});
