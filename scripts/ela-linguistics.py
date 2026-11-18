#!/usr/bin/env python3
"""Official spaCy/ERRANT workers. One invocation loads one spaCy pipeline.

T5 runs in a different Rust process and has already exited before ERRANT starts.
All models/packages must exist locally. No cloud APIs or runtime downloads.
"""

import json
import os
import sys
from collections import Counter, defaultdict


def analyze(nlp, text):
    doc = nlp(text)
    tokens = [token for token in doc if not token.is_space]
    words = [token for token in tokens if token.pos_ not in ("PUNCT", "SYM")]
    content = [
        token
        for token in words
        if token.pos_ in ("NOUN", "PROPN", "VERB", "ADJ", "ADV")
    ]
    morph = defaultdict(Counter)
    for token in tokens:
        for key, value in token.morph.to_dict().items():
            morph[key][value] += 1
    edges = [abs(token.i - token.head.i) for token in tokens if token.head != token]
    sentences = [
        sentence.text.strip() for sentence in doc.sents if sentence.text.strip()
    ]
    return {
        "language": "en",
        "sentences": len(sentences),
        "sentence_texts": sentences,
        "tokens": len(words),
        "content_words": len(content),
        "lexical_density_percent": 100 * len(content) / len(words) if words else None,
        "unique_lemmas": len({token.lemma_.lower() for token in words}),
        "content_lemmas": sorted({token.lemma_.lower() for token in content})[:500],
        "pos_counts": dict(Counter(token.pos_ for token in tokens)),
        "dependency_counts": dict(Counter(token.dep_ for token in tokens)),
        "morphological_features": {key: dict(values) for key, values in morph.items()},
        "mean_dependency_distance": sum(edges) / len(edges) if edges else None,
    }


def annotate(nlp, pairs):
    import errant

    annotator = errant.load("en", nlp)
    result = []
    for pair in pairs:
        edits = []
        if pair["status"] == "complete" and pair["original"] != pair["suggested"]:
            # Match ELA's annotator.parse defaults (whitespace tokenization).
            original = annotator.parse(pair["original"])
            corrected = annotator.parse(pair["suggested"])
            for edit in annotator.annotate(original, corrected):
                edits.append(
                    {
                        "category": edit.type,
                        "original": original[edit.o_start : edit.o_end].text,
                        "suggested": corrected[edit.c_start : edit.c_end].text,
                        "original_start": edit.o_start,
                        "original_end": edit.o_end,
                        "suggested_start": edit.c_start,
                        "suggested_end": edit.c_end,
                        "included_by_ela": edit.type
                        not in ("R:ORTH", "R:SPELL", "R:OTHER"),
                    }
                )
        result.append({**pair, "edits": edits})
    return result


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in ("analyze", "annotate"):
        raise SystemExit("Usage: ela-linguistics.py analyze|annotate REQUEST.json")
    if os.path.getsize(sys.argv[2]) > 256 * 1024:
        raise SystemExit("Linguistic request exceeds limit")
    with open(sys.argv[2], encoding="utf-8") as stream:
        request = json.load(stream)
    import torch

    torch.set_num_threads(2)
    torch.set_num_interop_threads(1)
    import spacy

    spacy.require_cpu()
    nlp = spacy.load(os.environ["ELA_SPACY_MODEL"])
    nlp.max_length = 40000
    if sys.argv[1] == "analyze":
        text = request["text"]
        if len(text) > 30000:
            raise SystemExit("Linguistic input exceeds limit")
        result = analyze(nlp, text)
    else:
        if len(request["pairs"]) > 16:
            raise SystemExit("Grammar sentence budget exceeded")
        result = annotate(nlp, request["pairs"])
    json.dump(result, sys.stdout, ensure_ascii=False)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
