// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

import { describe, expect, test } from "bun:test";

import {
    avancementOuvrage,
    estTranscrite,
    parLots,
    pourcent,
    totalise,
} from "./databooks-avancement";

describe("estTranscrite", () => {
    test("du texte compte", () => {
        expect(estTranscrite({ number: 1, text: "ブイジャンプ" })).toBe(true);
    });

    test("les trois façons de ne rien dire comptent pareil", () => {
        // `null` efface une transcription, `""` en enregistre une vide, et des
        // espaces viennent d'une planche muette déposée quand même. Un lecteur
        // ne lit rien dans les trois cas.
        expect(estTranscrite({ number: 1, text: null })).toBe(false);
        expect(estTranscrite({ number: 1, text: "" })).toBe(false);
        expect(estTranscrite({ number: 1, text: "  \n\t " })).toBe(false);
        expect(estTranscrite({ number: 1 })).toBe(false);
    });
});

describe("avancementOuvrage", () => {
    test("compte les planches lues sur le total", () => {
        const a = avancementOuvrage({
            id: 120,
            title: "V Jump Février 2019",
            category: "V-Jump",
            pages: [
                { number: 1, image: "a.jpg", text: "a" },
                { number: 2, image: "b.jpg", text: null },
                { number: 3, image: "c.jpg", text: "b" },
            ],
        });
        expect(a).toEqual({
            id: 120,
            titre: "V Jump Février 2019",
            categorie: "V-Jump",
            planches: 3,
            transcrites: 2,
            sansScan: 0,
        });
    });

    test("une planche sans scan ni texte sort du dénominateur", () => {
        // 228 lignes de ce genre dans le seul Daizenshuu 1 : une place réservée
        // pour un scan jamais arrivé. Les compter promettrait une fin
        // impossible — l'ouvrage plafonnerait à 2 %.
        const a = avancementOuvrage({
            id: 1,
            title: "Daizenshuu 1",
            pages: [
                { number: 1, image: "a.jpg", text: "lu" },
                { number: 2, image: null, text: null },
                { number: 3, image: null, text: null },
            ],
        });
        expect(a.planches).toBe(1);
        expect(a.transcrites).toBe(1);
        expect(a.sansScan).toBe(2);
    });

    test("une planche transcrite ailleurs compte, même sans scan en base", () => {
        // 236 planches portent du texte venu d'une autre source sans image.
        // Les sortir du dénominateur tout en comptant leur texte mettrait
        // l'ouvrage au-dessus de 100 %.
        const a = avancementOuvrage({
            id: 6,
            title: "Super Exciting Guide",
            pages: [
                { number: 1, image: null, text: "venu d'ailleurs" },
                { number: 2, image: "b.jpg", text: null },
            ],
        });
        expect(a.planches).toBe(2);
        expect(a.transcrites).toBe(1);
        expect(a.sansScan).toBe(0);
    });

    test("un ouvrage sans planche ni catégorie ne fait pas tomber le compte", () => {
        const a = avancementOuvrage({ id: 7, title: "Interview" });
        expect(a.planches).toBe(0);
        expect(a.sansScan).toBe(0);
        expect(a.transcrites).toBe(0);
        expect(a.categorie).toBe("(sans catégorie)");
    });
});

describe("totalise", () => {
    test("regroupe par catégorie, la plus fournie en tête", () => {
        const t = totalise([
            { id: 1, titre: "a", categorie: "V-Jump", planches: 40, transcrites: 4, sansScan: 0 },
            {
                id: 2,
                titre: "b",
                categorie: "Databook",
                planches: 200,
                transcrites: 200,
                sansScan: 3,
            },
            { id: 3, titre: "c", categorie: "V-Jump", planches: 30, transcrites: 0, sansScan: 1 },
        ]);
        expect(t).toEqual([
            { categorie: "Databook", ouvrages: 1, planches: 200, transcrites: 200, sansScan: 3 },
            { categorie: "V-Jump", ouvrages: 2, planches: 70, transcrites: 4, sansScan: 1 },
        ]);
    });
});

describe("pourcent", () => {
    test("arrondit à la décimale", () => {
        expect(pourcent(5302, 11775).trim()).toBe("45.0%");
    });

    test("un ouvrage sans planche n'est pas 0 % mais rien", () => {
        // Les cinq interviews n'ont aucune planche : les afficher à 0 % les
        // ferait passer pour du travail en retard alors qu'il n'y en a pas.
        expect(pourcent(0, 0).trim()).toBe("-");
    });
});

describe("parLots", () => {
    test("garde l'ordre des entrées malgré l'ordre d'exécution", async () => {
        const out = await parLots([5, 1, 3], 3, async (n) => {
            await Bun.sleep(n);
            return n * 2;
        });
        expect(out).toEqual([10, 2, 6]);
    });

    test("ne dépasse jamais la largeur demandée", async () => {
        let envol = 0;
        let pic = 0;
        await parLots(
            Array.from({ length: 20 }, (_, i) => i),
            3,
            async () => {
                envol += 1;
                pic = Math.max(pic, envol);
                await Bun.sleep(1);
                envol -= 1;
                return null;
            },
        );
        expect(pic).toBeLessThanOrEqual(3);
    });

    test("une liste vide ne bloque pas", async () => {
        expect(await parLots([], 8, async () => 1)).toEqual([]);
    });
});
