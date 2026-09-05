# SPDX-License-Identifier: Apache-2.0

"""GraphRAG entity and relation extraction module.

Identifies entities, relationships, strengths, and descriptions from chunks,
performs recursive entity/relationship merging, and builds structured graphs.
"""

import asyncio
import logging
import re
from collections import defaultdict
from collections.abc import Callable
from typing import Any

import networkx as nx

logger = logging.getLogger(__name__)

# Constants
DEFAULT_ENTITY_TYPES = ["organization", "person", "geo", "event", "category"]
DEFAULT_TUPLE_DELIMITER = "<|>"
DEFAULT_RECORD_DELIMITER = "##"
DEFAULT_COMPLETION_DELIMITER = "<|COMPLETE|>"

# Prompts
GRAPH_EXTRACTION_PROMPT = """
-Goal-
Given a text document that is potentially relevant to this activity and a list of entity types, identify all entities of those types from the text and all relationships among the identified entities.

-Steps-
1. Identify all entities. For each identified entity, extract the following information:
- entity_name: Name of the entity, capitalized, in language of 'Text'
- entity_type: One of the following types: [{entity_types}]
- entity_description: Comprehensive description of the entity's attributes and activities in language of 'Text'
Format each entity as ("entity"{tuple_delimiter}<entity_name>{tuple_delimiter}<entity_type>{tuple_delimiter}<entity_description>)

2. From the entities identified in step 1, identify all pairs of (source_entity, target_entity) that are *clearly related* to each other.
For each pair of related entities, extract the following information:
- source_entity: name of the source entity, as identified in step 1
- target_entity: name of the target entity, as identified in step 1
- relationship_description: explanation as to why you think the source entity and the target entity are related to each other in language of 'Text'
- relationship_strength: a numeric score indicating strength of the relationship between the source entity and target entity
 Format each relationship as ("relationship"{tuple_delimiter}<source_entity>{tuple_delimiter}<target_entity>{tuple_delimiter}<relationship_description>{tuple_delimiter}<relationship_strength>)

3. Return output as a single list of all the entities and relationships identified in steps 1 and 2. Use **{record_delimiter}** as the list delimiter.

4. When finished, output {completion_delimiter}

######################
-Examples-
######################
Example 1:

Entity_types: [person, technology, mission, organization, location]
Text:
while Alex clenched his jaw, the buzz of frustration dull against the backdrop of Taylor's authoritarian certainty. It was this competitive undercurrent that kept him alert, the sense that his and Jordan's shared commitment to discovery was an unspoken rebellion against Cruz's narrowing vision of control and order.

Then Taylor did something unexpected. They paused beside Jordan and, for a moment, observed the device with something akin to reverence. “If this tech can be understood..." Taylor said, their voice quieter, "It could change the game for us. For all of us.”

The underlying dismissal earlier seemed to falter, replaced by a glimpse of reluctant respect for the gravity of what lay in their hands. Jordan looked up, and for a fleeting heartbeat, their eyes locked with Taylor's, a wordless clash of wills softening into an uneasy truce.

It was a small transformation, barely perceptible, but one that Alex noted with an inward nod. They had all been brought here by different paths
################
Output:
("entity"{tuple_delimiter}"Alex"{tuple_delimiter}"person"{tuple_delimiter}"Alex is a character who experiences frustration and is observant of the dynamics among other characters."){record_delimiter}
("entity"{tuple_delimiter}"Taylor"{tuple_delimiter}"person"{tuple_delimiter}"Taylor is portrayed with authoritarian certainty and shows a moment of reverence towards a device, indicating a change in perspective."){record_delimiter}
("entity"{tuple_delimiter}"Jordan"{tuple_delimiter}"person"{tuple_delimiter}"Jordan shares a commitment to discovery and has a significant interaction with Taylor regarding a device."){record_delimiter}
("entity"{tuple_delimiter}"Cruz"{tuple_delimiter}"person"{tuple_delimiter}"Cruz is associated with a vision of control and order, influencing the dynamics among other characters."){record_delimiter}
("entity"{tuple_delimiter}"The Device"{tuple_delimiter}"technology"{tuple_delimiter}"The Device is central to the story, with potential game-changing implications, and is revered by Taylor."){record_delimiter}
("relationship"{tuple_delimiter}"Alex"{tuple_delimiter}"Taylor"{tuple_delimiter}"Alex is affected by Taylor's authoritarian certainty and observes changes in Taylor's attitude towards the device."{tuple_delimiter}7){record_delimiter}
("relationship"{tuple_delimiter}"Alex"{tuple_delimiter}"Jordan"{tuple_delimiter}"Alex and Jordan share a commitment to discovery, which contrasts with Cruz's vision."{tuple_delimiter}6){record_delimiter}
("relationship"{tuple_delimiter}"Taylor"{tuple_delimiter}"Jordan"{tuple_delimiter}"Taylor and Jordan interact directly regarding the device, leading to a moment of mutual respect and an uneasy truce."{tuple_delimiter}8){record_delimiter}
("relationship"{tuple_delimiter}"Jordan"{tuple_delimiter}"Cruz"{tuple_delimiter}"Jordan's commitment to discovery is in rebellion against Cruz's vision of control and order."{tuple_delimiter}5){record_delimiter}
("relationship"{tuple_delimiter}"Taylor"{tuple_delimiter}"The Device"{tuple_delimiter}"Taylor shows reverence towards the device, indicating its importance and potential impact."{tuple_delimiter}9){completion_delimiter}

-Real Data-
######################
Entity_types: {entity_types}
Text: {input_text}
######################
Output:"""

CONTINUE_PROMPT = "MANY entities were missed in the last extraction. Add them below using the same format:\n"
LOOP_PROMPT = "It appears some entities may have still been missed. Answer Y if there are still entities that need to be added, or N if there are none. Please answer with a single letter Y or N.\n"

SUMMARIZE_DESCRIPTIONS_PROMPT = """
You are a helpful assistant responsible for generating a comprehensive summary of the data provided below.
Given one or two entities, and a list of descriptions, all related to the same entity or group of entities.
Please concatenate all of these into a single, comprehensive description. Make sure to include information collected from all the descriptions.
If the provided descriptions are contradictory, please resolve the contradictions and provide a single, coherent summary.
Make sure it is written in third person, and include the entity names so we have the full context.
Use {language} as output language.

#######
-Data-
Entities: {entity_name}
Description List: {description_list}
#######
"""


class GraphExtractor:
    """GraphRAG Entity and Relationship extractor using an LLM client."""

    def __init__(
        self,
        llm_chat_fn: Callable[[str, list[dict[str, str]], dict[str, Any]], Any],
        entity_types: list[str] | None = None,
        language: str = "English",
        max_gleanings: int = 1,
    ):
        self._llm_chat_fn = llm_chat_fn
        self.entity_types = entity_types or DEFAULT_ENTITY_TYPES
        self.language = language
        self.max_gleanings = max_gleanings

    async def _chat(
        self,
        system: str,
        history: list[dict[str, str]],
        gen_conf: dict[str, Any],
    ) -> str:
        """Call the LLM chat function wrapper."""
        if asyncio.iscoroutinefunction(self._llm_chat_fn):
            response = await self._llm_chat_fn(system, history, gen_conf)
        else:
            response = self._llm_chat_fn(system, history, gen_conf)
        response = re.sub(r"^.*</think>", "", response, flags=re.DOTALL)
        return response

    @staticmethod
    def _clean_str(val: Any) -> str:
        """Clean string wrappers."""
        if not isinstance(val, str):
            val = str(val)
        val = val.strip()
        if val.startswith('"') and val.endswith('"'):
            val = val[1:-1]
        return val.strip()

    def _split_by_markers(self, text: str, markers: list[str]) -> list[str]:
        """Split text using multiple marker options."""
        if not markers:
            return [text]
        pattern = "|".join(re.escape(m) for m in markers)
        return [p.strip() for p in re.split(pattern, text) if p.strip()]

    def _parse_entity(
        self, record_attributes: list[str], chunk_id: str
    ) -> dict[str, Any] | None:
        """Parse attributes into entity metadata if record represents an entity."""
        if (
            len(record_attributes) < 4
            or self._clean_str(record_attributes[0]).lower() != "entity"
        ):
            return None

        name = self._clean_str(record_attributes[1]).upper()
        type_ = self._clean_str(record_attributes[2]).upper()
        desc = self._clean_str(record_attributes[3])

        if not name or not type_:
            return None

        return {
            "entity_name": name,
            "entity_type": type_,
            "description": desc,
            "source_id": chunk_id,
        }

    def _parse_relationship(
        self, record_attributes: list[str], chunk_id: str
    ) -> dict[str, Any] | None:
        """Parse attributes into relationship metadata if record represents a relationship."""
        if (
            len(record_attributes) < 5
            or self._clean_str(record_attributes[0]).lower() != "relationship"
        ):
            return None

        src = self._clean_str(record_attributes[1]).upper()
        tgt = self._clean_str(record_attributes[2]).upper()
        desc = self._clean_str(record_attributes[3])

        strength_str = self._clean_str(record_attributes[4])
        try:
            strength = float(strength_str)
        except ValueError:
            strength = 1.0

        if not src or not tgt:
            return None

        pair = sorted([src, tgt])
        return {
            "src_id": pair[0],
            "tgt_id": pair[1],
            "weight": strength,
            "description": desc,
            "source_id": chunk_id,
        }

    async def _extract_from_chunk(
        self, chunk_id: str, content: str
    ) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
        """Extract nodes and edges from a single content chunk."""
        prompt = GRAPH_EXTRACTION_PROMPT.format(
            entity_types=",".join(self.entity_types),
            tuple_delimiter=DEFAULT_TUPLE_DELIMITER,
            record_delimiter=DEFAULT_RECORD_DELIMITER,
            completion_delimiter=DEFAULT_COMPLETION_DELIMITER,
            input_text=content,
        )

        response = await self._chat(
            prompt,
            [
                {
                    "role": "user",
                    "content": "Extract entities and relationships from the text.",
                }
            ],
            {},
        )
        results = response or ""

        history = [
            {"role": "system", "content": prompt},
            {
                "role": "user",
                "content": "Extract entities and relationships from the text.",
            },
            {"role": "assistant", "content": response},
        ]

        for _ in range(self.max_gleanings):
            history.append({"role": "user", "content": CONTINUE_PROMPT})
            response = await self._chat("", history, {})
            results += response or ""
            history.append({"role": "assistant", "content": response})

            history.append({"role": "user", "content": LOOP_PROMPT})
            continuation = await self._chat("", history, {})
            if "Y" not in continuation.upper():
                break
            history.append({"role": "assistant", "content": "Y"})

        records = self._split_by_markers(
            results, [DEFAULT_RECORD_DELIMITER, DEFAULT_COMPLETION_DELIMITER]
        )

        entities = []
        relationships = []

        for record in records:
            match = re.search(r"\((.*)\)", record)
            if match is None:
                continue
            record_str = match.group(1)
            attributes = self._split_by_markers(
                record_str, [DEFAULT_TUPLE_DELIMITER]
            )

            ent = self._parse_entity(attributes, chunk_id)
            if ent:
                entities.append(ent)
                continue

            rel = self._parse_relationship(attributes, chunk_id)
            if rel:
                relationships.append(rel)

        return entities, relationships

    async def _merge_entity_descriptions(
        self, entity_name: str, entities: list[dict[str, Any]]
    ) -> dict[str, Any]:
        """Summarize multiple extracted entities with the same name into a unified description."""
        if not entities:
            return {}

        first = entities[0]
        if len(entities) == 1:
            return {
                "entity_name": entity_name,
                "entity_type": first["entity_type"],
                "description": first["description"],
                "source_id": [first["source_id"]],
            }

        descriptions = [
            f"- {e['description']}" for e in entities if e.get("description")
        ]
        description_list = "\n".join(descriptions)

        prompt = SUMMARIZE_DESCRIPTIONS_PROMPT.format(
            language=self.language,
            entity_name=entity_name,
            description_list=description_list,
        )

        summary = await self._chat(
            prompt,
            [
                {
                    "role": "user",
                    "content": "Summarize the descriptions into one cohesive paragraph.",
                }
            ],
            {},
        )
        return {
            "entity_name": entity_name,
            "entity_type": first["entity_type"],
            "description": summary.strip(),
            "source_id": list(set(e["source_id"] for e in entities)),
        }

    async def _merge_relationship_descriptions(
        self, src: str, tgt: str, rels: list[dict[str, Any]]
    ) -> dict[str, Any]:
        """Summarize and combine multiple edges between the same two entities."""
        if not rels:
            return {}

        if len(rels) == 1:
            r = rels[0]
            return {
                "src_id": src,
                "tgt_id": tgt,
                "weight": r["weight"],
                "description": r["description"],
                "source_id": [r["source_id"]],
            }

        total_weight = sum(r["weight"] for r in rels)
        avg_weight = total_weight / len(rels)

        descriptions = [
            f"- {r['description']}" for r in rels if r.get("description")
        ]
        description_list = "\n".join(descriptions)

        prompt = SUMMARIZE_DESCRIPTIONS_PROMPT.format(
            language=self.language,
            entity_name=f"{src} and {tgt}",
            description_list=description_list,
        )

        summary = await self._chat(
            prompt,
            [{"role": "user", "content": "Summarize the relationships."}],
            {},
        )
        return {
            "src_id": src,
            "tgt_id": tgt,
            "weight": avg_weight,
            "description": summary.strip(),
            "source_id": list(set(r["source_id"] for r in rels)),
        }

    async def extract_graph(
        self, chunks: list[str], doc_id: str = "doc"
    ) -> nx.Graph:
        """Process chunks to extract entities, relationships, merge them, and output a NetworkX graph."""
        extracted_entities = defaultdict(list)
        extracted_relationships = defaultdict(list)

        tasks = []
        for idx, chunk_content in enumerate(chunks):
            chunk_id = f"{doc_id}_chunk_{idx}"
            tasks.append(self._extract_from_chunk(chunk_id, chunk_content))

        results = await asyncio.gather(*tasks)

        for entities, relationships in results:
            for ent in entities:
                extracted_entities[ent["entity_name"]].append(ent)
            for rel in relationships:
                key = (rel["src_id"], rel["tgt_id"])
                extracted_relationships[key].append(rel)

        merge_entity_tasks = []
        for name, ents in extracted_entities.items():
            merge_entity_tasks.append(
                self._merge_entity_descriptions(name, ents)
            )

        merged_entities = await asyncio.gather(*merge_entity_tasks)

        merge_rel_tasks = []
        for (src, tgt), rels in extracted_relationships.items():
            merge_rel_tasks.append(
                self._merge_relationship_descriptions(src, tgt, rels)
            )

        merged_relationships = await asyncio.gather(*merge_rel_tasks)

        g = nx.Graph()
        for ent in merged_entities:
            if not ent:
                continue
            g.add_node(
                ent["entity_name"],
                entity_type=ent["entity_type"],
                description=ent["description"],
                source_id=ent["source_id"],
            )

        for rel in merged_relationships:
            if not rel:
                continue
            g.add_edge(
                rel["src_id"],
                rel["tgt_id"],
                weight=rel["weight"],
                description=rel["description"],
                source_id=rel["source_id"],
            )

        return g
