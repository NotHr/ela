"use strict";

const $ = (selector) => document.querySelector(selector);
const strings = {
  en: {
    overview: "Overview",
    assessments: "Assessments",
    models: "Models",
    local: "Local workspace",
    offline: "Disconnected",
    interface: "Interface language",
    export: "Export",
    newAssessment: "New assessment",
    recent: "Recent assessments",
    viewAll: "View all",
    learner: "Learner",
    classroom: "Class",
    language: "Language",
    status: "Status",
    readingScore: "Reading match",
    date: "Date",
    actions: "Actions",
    search: "Search assessments",
    allStatuses: "All statuses",
    allLanguages: "All languages",
    complete: "Completed",
    processing: "Processing",
    failed: "Failed",
    previous: "Previous",
    next: "Next",
    legacyRecords: "Previous manual records",
    task: "Activity",
    readAloud: "Read aloud",
    speaking: "Free speaking",
    activityName: "Activity name",
    passage: "Reading passage",
    dropAudio: "Drop audio or browse",
    cancel: "Cancel",
    assess: "Start assessment",
    close: "Close",
    delete: "Delete",
    total: "Assessments",
    learners: "Learners",
    needsSupport: "Needs support",
    empty: "No assessments",
    noMatch: "No results",
    of: "of",
    accepted: "Accepted",
    converting: "Converting audio",
    audio_checked: "Audio checked",
    transcribing: "Transcribing",
    analyzing: "Analyzing",
    reporting: "Saving",
    viewReport: "View report",
    provisional: "Provisional",
    on_track: "On track",
    developing: "Developing",
    needs_support: "Needs support",
    insufficient_evidence: "Insufficient evidence",
    uncertain_transcription: "Uncertain transcript",
    measured: "Analyzed",
    recognized: "Recognized words",
    pace: "Matched words / min",
    observedPace: "Words / min",
    duration: "Duration (s)",
    wer: "Word differences (%)",
    transcript: "Transcript",
    focusWords: "Practice words",
    practice: "Personalised practice",
    intervention: "Teacher interventions",
    diagnostics: "Details",
    modelReady: "Model configured",
    modelMissing: "Model not configured",
    configured: "Configured",
    notConfigured: "Not configured",
    checking: "Checking…",
    backend: "Runtime",
    native: "sherpa-onnx · CPU",
    external: "Local adapter",
    configuration: "Configuration",
    converter: "Audio conversion",
    available: "Available",
    unavailable: "Unavailable",
    networkError: "Connection failed",
    submitted: "Assessment started",
    deleted: "Assessment deleted",
    deleteConfirm: "Delete this assessment?",
    tooLarge: "Upload exceeds 64 MB",
    listen_and_repeat_focus_words: "Listen and repeat practice words",
    blend_focus_words: "Blend sounds and syllables",
    reread_same_passage: "Reread the passage",
    reread_for_expression: "Reread with expression",
    try_next_passage: "Try another passage",
    verify_with_short_one_to_one_reading:
      "Check with a short individual reading",
    model_then_echo_read: "Model and echo-read",
    guided_small_group_reading: "Guided small-group reading",
    check_focus_words_in_context: "Practise target words in context",
    extend_with_another_passage: "Introduce another passage",
    asr_is_not_ground_truth: "ASR uncertainty",
    learner_only_audio_required: "Learner-only audio",
    low_audio_energy: "Low audio signal",
    no_recognized_words: "No recognized words",
    repetition_may_be_asr_error_or_real_speech: "Repetition detected",
    audio_clipping: "Audio clipping",
    asr_window_boundaries_may_cut_words: "Windowed transcription",
    reference_comparison_limit_or_empty_reference:
      "Reference comparison unavailable",
    reference_errors_include_asr_errors_not_pronunciation:
      "Reference differences include ASR errors",
    provisional_reading_screen_not_validated_proficiency:
      "Unvalidated screening thresholds",
    record_closer_to_microphone: "Record closer to the microphone",
    collect_another_speech_sample: "Record another sample",
    reduce_recording_gain: "Reduce microphone gain",
    practice_reference_then_record_again: "Practise and rerecord",
    invite_a_longer_response: "Try a longer response",
    compare_with_future_samples_same_activity:
      "Reassess with the same activity",
    noScore: "—",
    ela_language_id_matrix_not_integrated:
      "ELA language scoring matrix unavailable",
    grammar_and_pos_models_not_integrated: "Grammar analysis unavailable",
    speaker_diarization_not_integrated: "Speaker separation unavailable",
    comprehension_not_measured: "Comprehension unmeasured",
  },
  te: {
    overview: "సారాంశం",
    assessments: "మూల్యాంకనాలు",
    models: "మోడళ్లు",
    local: "స్థానిక కార్యస్థలం",
    offline: "కనెక్షన్ లేదు",
    interface: "ఇంటర్‌ఫేస్ భాష",
    export: "ఎగుమతి",
    newAssessment: "కొత్త మూల్యాంకనం",
    recent: "ఇటీవలి మూల్యాంకనాలు",
    viewAll: "అన్నీ చూడండి",
    learner: "విద్యార్థి",
    classroom: "తరగతి",
    language: "భాష",
    status: "స్థితి",
    readingScore: "పఠన సరిపోలిక",
    date: "తేదీ",
    actions: "చర్యలు",
    search: "మూల్యాంకనాలను వెతకండి",
    allStatuses: "అన్ని స్థితులు",
    allLanguages: "అన్ని భాషలు",
    complete: "పూర్తయింది",
    processing: "ప్రాసెస్ అవుతోంది",
    failed: "విఫలమైంది",
    previous: "మునుపటి",
    next: "తదుపరి",
    legacyRecords: "పాత మానవ మూల్యాంకనాలు",
    task: "కార్యాచరణ",
    readAloud: "బిగ్గరగా చదవడం",
    speaking: "స్వేచ్ఛగా మాట్లాడటం",
    activityName: "కార్యాచరణ పేరు",
    passage: "చదవవలసిన పాఠం",
    dropAudio: "ఆడియో వదలండి లేదా ఎంచుకోండి",
    cancel: "రద్దు",
    assess: "మూల్యాంకనం ప్రారంభించండి",
    close: "మూసివేయండి",
    delete: "తొలగించండి",
    total: "మూల్యాంకనాలు",
    learners: "విద్యార్థులు",
    needsSupport: "సహాయం అవసరం",
    empty: "మూల్యాంకనాలు లేవు",
    noMatch: "ఫలితాలు లేవు",
    of: "మొత్తం",
    accepted: "స్వీకరించబడింది",
    converting: "ఆడియో మార్పిడి",
    audio_checked: "ఆడియో తనిఖీ పూర్తయింది",
    transcribing: "మాటల గుర్తింపు",
    analyzing: "విశ్లేషణ",
    reporting: "భద్రపరుస్తున్నాము",
    viewReport: "నివేదిక చూడండి",
    provisional: "తాత్కాలిక అంచనా",
    on_track: "సరైన దిశలో",
    developing: "మెరుగుపడుతోంది",
    needs_support: "సహాయం అవసరం",
    insufficient_evidence: "తగిన ఆధారాలు లేవు",
    uncertain_transcription: "అనిశ్చిత లిపి",
    measured: "విశ్లేషించబడింది",
    recognized: "గుర్తించిన పదాలు",
    pace: "సరిపోలిన పదాలు / నిమిషం",
    observedPace: "పదాలు / నిమిషం",
    duration: "వ్యవధి (సె.)",
    wer: "పదాల తేడా (%)",
    transcript: "గుర్తించిన మాటలు",
    focusWords: "సాధన పదాలు",
    practice: "వ్యక్తిగత సాధన",
    intervention: "ఉపాధ్యాయ చర్యలు",
    diagnostics: "వివరాలు",
    modelReady: "మోడల్ అమర్చబడింది",
    modelMissing: "మోడల్ అమర్చలేదు",
    configured: "అమర్చబడింది",
    notConfigured: "అమర్చలేదు",
    checking: "తనిఖీ జరుగుతోంది…",
    backend: "రన్‌టైమ్",
    native: "sherpa-onnx · CPU",
    external: "స్థానిక అడాప్టర్",
    configuration: "అమరిక",
    converter: "ఆడియో మార్పిడి",
    available: "అందుబాటులో ఉంది",
    unavailable: "అందుబాటులో లేదు",
    networkError: "కనెక్షన్ విఫలమైంది",
    submitted: "మూల్యాంకనం ప్రారంభమైంది",
    deleted: "మూల్యాంకనం తొలగించబడింది",
    deleteConfirm: "ఈ మూల్యాంకనాన్ని తొలగించాలా?",
    tooLarge: "అప్‌లోడ్ 64 MB మించిపోయింది",
    listen_and_repeat_focus_words: "సాధన పదాలను విని పలకండి",
    blend_focus_words: "ధ్వనులు, అక్షరాలను కలిపి చదవండి",
    reread_same_passage: "అదే పాఠాన్ని మళ్లీ చదవండి",
    reread_for_expression: "భావంతో మళ్లీ చదవండి",
    try_next_passage: "మరో పాఠాన్ని ప్రయత్నించండి",
    verify_with_short_one_to_one_reading: "వ్యక్తిగత పఠనంతో నిర్ధారించండి",
    model_then_echo_read: "చదివి వినిపించి అనుకరించనివ్వండి",
    guided_small_group_reading: "చిన్న బృందంలో పఠన సాధన",
    check_focus_words_in_context: "సందర్భంలో సాధన పదాలను చదివించండి",
    extend_with_another_passage: "మరో పాఠాన్ని పరిచయం చేయండి",
    asr_is_not_ground_truth: "మాటల గుర్తింపులో అనిశ్చితి",
    learner_only_audio_required: "విద్యార్థి ఆడియో మాత్రమే",
    low_audio_energy: "తక్కువ ఆడియో సిగ్నల్",
    no_recognized_words: "పదాలు గుర్తించబడలేదు",
    repetition_may_be_asr_error_or_real_speech: "పునరావృతం గుర్తించబడింది",
    audio_clipping: "ఆడియో క్లిప్పింగ్",
    asr_window_boundaries_may_cut_words: "భాగాలుగా లిప్యంతరీకరణ",
    reference_comparison_limit_or_empty_reference:
      "పాఠంతో పోలిక అందుబాటులో లేదు",
    reference_errors_include_asr_errors_not_pronunciation:
      "తేడాల్లో మోడల్ లోపాలు కూడా ఉంటాయి",
    provisional_reading_screen_not_validated_proficiency:
      "ధృవీకరించని అంచనా పరిమితులు",
    record_closer_to_microphone: "మైక్రోఫోన్‌కు దగ్గరగా రికార్డు చేయండి",
    collect_another_speech_sample: "మరో నమూనా రికార్డు చేయండి",
    reduce_recording_gain: "మైక్రోఫోన్ గెయిన్ తగ్గించండి",
    practice_reference_then_record_again: "సాధన చేసి మళ్లీ రికార్డు చేయండి",
    invite_a_longer_response: "మరింత మాట్లాడనివ్వండి",
    compare_with_future_samples_same_activity:
      "అదే కార్యాచరణతో మళ్లీ అంచనా వేయండి",
    noScore: "—",
    ela_language_id_matrix_not_integrated: "ELA భాష స్కోరింగ్ పట్టిక లేదు",
    grammar_and_pos_models_not_integrated: "వ్యాకరణ విశ్లేషణ లేదు",
    speaker_diarization_not_integrated: "వక్తల విభజన లేదు",
    comprehension_not_measured: "అవగాహన కొలవలేదు",
  },
};
Object.assign(strings.en, {
  recordingMode: "Recording",
  conversation: "Teacher + student",
  studentOnly: "Student only",
  languageMode: "Language switching",
  autoLanguages: "Auto · Telugu + English",
  fixedLanguage: "Fixed language",
  studentReference: "Student voice sample (3–30 s)",
  diarization: "Speaker separation",
  languageDetection: "Language detection",
  processing_speech: "Processing student speech",
  speechSegments: "Student speech segments",
  time: "Time",
  studentSeconds: "Student audio (s)",
  speakers: "Speakers",
  voiceSimilarity: "Voice similarity",
  overlapSeconds: "Excluded overlap (s)",
  unknownSeconds: "Untranscribed student audio (s)",
  collect_clear_student_reference: "Use a clear student-only voice sample",
  empty_asr_window_excluded: "Unrecognized speech window excluded",
  language_candidates_restricted_te_en: "Language candidates: Telugu / English",
  partial_student_audio: "Partial student audio · score withheld",
  student_speaker_ambiguous: "Student voice match ambiguous",
  student_speech_not_verified: "Student speech not verified",
  overlapping_speech_excluded: "Overlapping speech excluded",
  unverified_student_turns_excluded: "Unverified student turns excluded",
  short_language_window_excluded: "Language window too short",
  unsupported_language_excluded: "Unsupported language excluded",
  speaker_and_language_models_can_be_wrong:
    "Speaker/language estimates may be wrong",
  windowed_language_detection_not_word_level:
    "Language switching is window-level, not word-level",
  incomplete_student_audio_score_withheld:
    "Incomplete student audio · score withheld",
});
Object.assign(strings.te, {
  recordingMode: "రికార్డింగ్",
  conversation: "ఉపాధ్యాయుడు + విద్యార్థి",
  studentOnly: "విద్యార్థి మాత్రమే",
  languageMode: "భాష మార్పు",
  autoLanguages: "స్వయంచాలకం · తెలుగు + English",
  fixedLanguage: "ఎంచుకున్న భాష",
  studentReference: "విద్యార్థి స్వర నమూనా (3–30 సె.)",
  diarization: "వక్తల విభజన",
  languageDetection: "భాష గుర్తింపు",
  processing_speech: "విద్యార్థి మాటల విశ్లేషణ",
  speechSegments: "విద్యార్థి మాటల భాగాలు",
  time: "సమయం",
  studentSeconds: "విద్యార్థి ఆడియో (సె.)",
  speakers: "వక్తలు",
  voiceSimilarity: "స్వర సారూప్యత",
  overlapSeconds: "తొలగించిన కలిసిన మాటలు (సె.)",
  unknownSeconds: "గుర్తించని విద్యార్థి ఆడియో (సె.)",
  collect_clear_student_reference: "స్పష్టమైన విద్యార్థి స్వర నమూనా ఇవ్వండి",
  empty_asr_window_excluded: "గుర్తించని మాటల భాగం తొలగించబడింది",
  language_candidates_restricted_te_en: "భాషలు: తెలుగు / English",
  partial_student_audio: "పాక్షిక విద్యార్థి ఆడియో · స్కోరు ఇవ్వలేదు",
  student_speaker_ambiguous: "విద్యార్థి స్వరంపై అనిశ్చితి",
  student_speech_not_verified: "విద్యార్థి మాటలు నిర్ధారించలేదు",
  overlapping_speech_excluded: "కలిసి మాట్లాడిన భాగాలు తొలగించబడ్డాయి",
  unverified_student_turns_excluded: "నిర్ధారించని భాగాలు తొలగించబడ్డాయి",
  short_language_window_excluded: "భాష గుర్తింపుకు భాగం చిన్నది",
  unsupported_language_excluded: "మద్దతులేని భాష తొలగించబడింది",
  speaker_and_language_models_can_be_wrong:
    "వక్త, భాష అంచనాల్లో లోపాలు ఉండవచ్చు",
  windowed_language_detection_not_word_level:
    "భాష గుర్తింపు భాగాల వారీగా; పదాల వారీగా కాదు",
  incomplete_student_audio_score_withheld:
    "విద్యార్థి ఆడియో అసంపూర్ణం · స్కోరు ఇవ్వలేదు",
});
Object.assign(strings.en, {
  students: "Students",
  studentId: "Student ID",
  searchStudents: "Search students",
  importCollection: "Import",
  collectionZip: "Collection ZIP",
  downloadForm: "Download mobile form",
  passageSource: "Passage source",
  paragraphPhoto: "Paragraph photo",
  typedPassage: "Text",
  extracting_text: "Reading photo",
  linguistic_analysis: "Linguistic analysis",
  uncertain_reference: "Uncertain passage photo",
  uncertain_ocr_reference_score_withheld: "Uncertain OCR · score withheld",
  retake_paragraph_photo: "Retake a clear paragraph photo",
  grammar_error_model_not_integrated: "Grammar error model unavailable",
  accepted: "Accepted",
  duplicate: "Already imported",
  invalid: "Invalid submission",
  nlpAnalysis: "Language analysis",
  lexicalDensity: "Lexical density (%)",
  lemmas: "Distinct lemmas",
  sentences: "Sentences",
  nlpUnavailable: "NLP model unavailable",
  nlp_on_asr_not_grammar_diagnosis: "ASR-based linguistic observations",
  read_aloud_grammar_belongs_to_passage_not_child_composition:
    "Read-aloud syntax is not authored grammar",
  model_quality_requires_child_language_validation:
    "Child-language validation needed",
  ocrConfidence: "OCR confidence",
  studentHistory: "Assessment history",
  comparablePassage: "Same passage & model",
  changePp: "Match change (pp)",
  paceChange: "Pace change",
  noSeries: "No scored readings",
  unknownModel: "Model version unrecorded",
  readingTrend: "Reading match (%)",
  assessmentCount: "Assessments",
  completedCount: "Completed",
  importStarted: "Collection accepted",
});
Object.assign(strings.te, {
  students: "విద్యార్థులు",
  studentId: "విద్యార్థి గుర్తింపు సంఖ్య",
  searchStudents: "విద్యార్థులను వెతకండి",
  importCollection: "దిగుమతి",
  collectionZip: "సేకరణ ZIP",
  downloadForm: "మొబైల్ ఫారం పొందండి",
  passageSource: "పాఠం మూలం",
  paragraphPhoto: "పాఠం ఫోటో",
  typedPassage: "వచనం",
  extracting_text: "ఫోటో నుంచి వచనం",
  linguistic_analysis: "భాషా విశ్లేషణ",
  uncertain_reference: "పాఠం ఫోటోపై అనిశ్చితి",
  uncertain_ocr_reference_score_withheld: "OCR అనిశ్చితి · స్కోరు ఇవ్వలేదు",
  retake_paragraph_photo: "పాఠాన్ని స్పష్టంగా మళ్లీ ఫోటో తీయండి",
  grammar_error_model_not_integrated: "వ్యాకరణ లోపాల మోడల్ లేదు",
  accepted: "స్వీకరించబడింది",
  duplicate: "ఇప్పటికే దిగుమతైంది",
  invalid: "చెల్లని సమర్పణ",
  nlpAnalysis: "భాషా విశ్లేషణ",
  lexicalDensity: "పద సాంద్రత (%)",
  lemmas: "వేర్వేరు మూలపదాలు",
  sentences: "వాక్యాలు",
  nlpUnavailable: "NLP మోడల్ లేదు",
  nlp_on_asr_not_grammar_diagnosis: "మాటల గుర్తింపు ఆధారిత పరిశీలనలు",
  read_aloud_grammar_belongs_to_passage_not_child_composition:
    "పఠన నిర్మాణం విద్యార్థి స్వీయ వ్యాకరణం కాదు",
  model_quality_requires_child_language_validation:
    "పిల్లల భాషపై ధృవీకరణ అవసరం",
  ocrConfidence: "OCR అంచనా",
  studentHistory: "మూల్యాంకన చరిత్ర",
  comparablePassage: "అదే పాఠం, మోడల్",
  changePp: "సరిపోలిక మార్పు",
  paceChange: "వేగం మార్పు",
  noSeries: "స్కోరు ఉన్న పఠనాలు లేవు",
  unknownModel: "మోడల్ సంచిక నమోదు కాలేదు",
  readingTrend: "పఠన సరిపోలిక (%)",
  assessmentCount: "మూల్యాంకనాలు",
  completedCount: "పూర్తయినవి",
  importStarted: "సేకరణ స్వీకరించబడింది",
});
Object.assign(strings.en, {
  nlpTelugu: "Telugu NLP",
  nlpEnglish: "English NLP",
  morphology: "Morphology & dependencies",
  dependencyDistance: "Dependency distance",
  lemmaCoverage: "Reference lemma coverage (%)",
  nlp_model_unavailable_te: "Telugu NLP unavailable",
  nlp_model_unavailable_en: "English NLP unavailable",
  nlp_failed_te: "Telugu parser failed",
  nlp_failed_en: "English parser failed",
  mixed_reference_not_dependency_parsed: "Mixed reference parse skipped",
  reference_nlp_failed: "Reference parser failed",
  nlp_unit_limit: "NLP segment limit",
});
Object.assign(strings.te, {
  nlpTelugu: "తెలుగు NLP",
  nlpEnglish: "ఆంగ్ల NLP",
  morphology: "పదరూపాలు, వాక్య సంబంధాలు",
  dependencyDistance: "వాక్య సంబంధ దూరం",
  lemmaCoverage: "పాఠం మూలపదాల సరిపోలిక (%)",
  nlp_model_unavailable_te: "తెలుగు NLP లేదు",
  nlp_model_unavailable_en: "ఆంగ్ల NLP లేదు",
  nlp_failed_te: "తెలుగు పార్సర్ విఫలం",
  nlp_failed_en: "ఆంగ్ల పార్సర్ విఫలం",
  mixed_reference_not_dependency_parsed: "మిశ్రమ పాఠం పార్సింగ్ చేయలేదు",
  reference_nlp_failed: "పాఠం పార్సింగ్ విఫలం",
  nlp_unit_limit: "NLP భాగాల పరిమితి",
});
Object.assign(strings.en, {
  practiceSlm: "Report SLM (required)",
  summary: "What this result means",
  grammar_analysis: "Checking possible English corrections",
  grammarTitle: "Possible English corrections",
  grammarNotice:
    "Suggestions about the recognized transcript—not confirmed mistakes by the child.",
  grammarOriginal: "Recognized text",
  grammarSuggested: "Suggested wording",
  grammarNoChanges:
    "No categorized grammar edits were proposed in the reviewed sentences. This does not prove that all speech was grammatically correct.",
  grammarUnavailable: "English correction check unavailable",
  grammarMemoryBudget: "English correction check needs more available memory",
  nlp_memory_budget_en:
    "spaCy needs about 3 GiB available memory; free memory and retry",
  grammar_proposals_on_asr_not_confirmed_child_errors:
    "Proposals concern recognized text, not confirmed learner errors",
  rust_t5_greedy_not_ela_beam_search:
    "Rust T5 uses greedy decoding, not ELA's five-beam search",
  grammar_sentence_or_token_budget: "Sentence/token budget limited the review",
  grammar_incomplete_sentence_analysis: "Sentence analysis was incomplete",
  grammarPartial: "Only part of the recording was reviewed",
  grammarReviewed: "Sentences reviewed",
  grammarReadAloud:
    "In read-aloud, these changes concern the recognized passage—not the child's authored grammar.",
  updateLinguistics: "Update language analysis",
  grammarSva: "Check agreement between subject and verb",
  grammarTense: "Check verb tense",
  grammarVerbForm: "Check verb form",
  grammarDeterminer: "Check an article or determiner (a, an, the)",
  grammarNumber: "Check singular or plural noun form",
  grammarPreposition: "Check the preposition",
  grammarOrder: "Check word order",
  grammarOther: "Review this possible wording change",
  grammarModel: "English T5 + spaCy / ERRANT",
  practice_subject_verb_agreement:
    "Practise short pairs such as ‘I go’ and ‘she goes’",
  practice_verb_tense:
    "Describe one activity yesterday and today, noticing the verb forms",
  practice_noun_number:
    "Practise singular/plural pairs such as ‘one book’ and ‘two books’",
  practice_articles: "Practise ‘a’, ‘an’ and ‘the’ in short sentences",
  practice_prepositions:
    "Describe where familiar objects are using ‘in’, ‘on’ and ‘under’",
  describe_familiar_picture: "Describe a familiar picture in 3–4 sentences",
  retell_short_story:
    "Listen to a short story and retell its beginning, middle and end",
  speak_connected_sentences:
    "Practise connecting sentences about one familiar topic",
  generateSummary: "Generate summary",
  summaryMissing: "A required summary has not been generated for this report.",
  writing_summary: "Writing summary",
  slm_summary_pending: "Summary pending",
  planning_practice: "Prioritizing practice",
  slm_disabled: "Rule-based suggestions",
  slm_insufficient_evidence: "SLM skipped · insufficient evidence",
  slm_model_unavailable: "SLM not configured",
  slm_model_too_large: "SLM exceeds model budget",
  slm_invalid_model: "SLM model unavailable",
  slm_memory_budget: "Summary unavailable · free more memory and retry",
  slm_context_budget: "Summary unavailable · input budget exceeded",
  slm_failed_or_unvalidated_output:
    "Summary generation failed validation · retry",
});
Object.assign(strings.te, {
  practiceSlm: "నివేదిక SLM (అవసరం)",
  summary: "ఈ ఫలితం ఏమి చెబుతోంది",
  grammar_analysis: "సంభావ్య ఆంగ్ల దిద్దుబాట్ల పరిశీలన",
  grammarTitle: "సంభావ్య ఆంగ్ల దిద్దుబాట్లు",
  grammarNotice:
    "ఇవి గుర్తించిన పాఠ్యానికి సూచనలు మాత్రమే; పిల్లల నిర్ధారిత తప్పులు కావు.",
  grammarOriginal: "గుర్తించిన పాఠ్యం",
  grammarSuggested: "సూచించిన వాక్యరూపం",
  grammarNoChanges:
    "పరిశీలించిన వాక్యాల్లో వర్గీకరించిన వ్యాకరణ మార్పులు సూచించలేదు. మాట్లాడినదంతా వ్యాకరణపరంగా సరైనదని ఇది నిరూపించదు.",
  grammarUnavailable: "ఆంగ్ల దిద్దుబాటు పరిశీలన అందుబాటులో లేదు",
  grammarMemoryBudget: "ఆంగ్ల దిద్దుబాటు పరిశీలనకు మరింత ఖాళీ మెమరీ అవసరం",
  nlp_memory_budget_en:
    "spaCyకి సుమారు 3 GiB ఖాళీ మెమరీ అవసరం; మెమరీ ఖాళీ చేసి మళ్లీ ప్రయత్నించండి",
  grammar_proposals_on_asr_not_confirmed_child_errors:
    "ఇవి గుర్తించిన పాఠ్యానికి సూచనలు; పిల్లల నిర్ధారిత తప్పులు కావు",
  rust_t5_greedy_not_ela_beam_search:
    "Rust T5లో greedy decoding ఉంది; ELA ఐదు-beam పద్ధతి కాదు",
  grammar_sentence_or_token_budget:
    "వాక్యాలు/టోకెన్ల పరిమితితో పరిశీలన పరిమితమైంది",
  grammar_incomplete_sentence_analysis: "వాక్య విశ్లేషణ అసంపూర్ణంగా ఉంది",
  grammarPartial: "రికార్డింగ్‌లో కొంత భాగాన్నే పరిశీలించారు",
  grammarReviewed: "పరిశీలించిన వాక్యాలు",
  grammarReadAloud:
    "పఠనంలో ఈ మార్పులు గుర్తించిన పేరాకు సంబంధించినవి; పిల్లల సొంత వాక్య నిర్మాణానికి కాదు.",
  updateLinguistics: "భాషా విశ్లేషణను నవీకరించండి",
  grammarSva: "కర్త, క్రియల మధ్య సరిపోలిక చూడండి",
  grammarTense: "క్రియా కాలాన్ని పరిశీలించండి",
  grammarVerbForm: "క్రియా రూపాన్ని పరిశీలించండి",
  grammarDeterminer: "ఆర్టికల్ లేదా నిర్ణాయక పదాన్ని పరిశీలించండి",
  grammarNumber: "నామవాచక ఏకవచన లేదా బహువచన రూపాన్ని పరిశీలించండి",
  grammarPreposition: "విభక్తి సంబంధ పదాన్ని పరిశీలించండి",
  grammarOrder: "పదాల క్రమాన్ని పరిశీలించండి",
  grammarOther: "ఈ సంభావ్య వాక్య మార్పును పరిశీలించండి",
  grammarModel: "ఆంగ్ల T5 + spaCy / ERRANT",
  practice_subject_verb_agreement:
    "‘I go’, ‘she goes’ వంటి చిన్న ఆంగ్ల వాక్య జంటలను సాధన చేయండి",
  practice_verb_tense:
    "నిన్న, ఈ రోజు చేసిన పనిని చెబుతూ ఆంగ్ల క్రియా రూపాలను గమనించండి",
  practice_noun_number:
    "‘one book’, ‘two books’ వంటి ఏకవచన/బహువచన జంటలను సాధన చేయండి",
  practice_articles:
    "చిన్న ఆంగ్ల వాక్యాల్లో ‘a’, ‘an’, ‘the’ ఉపయోగించడం సాధన చేయండి",
  practice_prepositions:
    "తెలిసిన వస్తువుల స్థానాన్ని ‘in’, ‘on’, ‘under’ ఉపయోగించి చెప్పండి",
  describe_familiar_picture: "తెలిసిన చిత్రం గురించి 3–4 వాక్యాల్లో చెప్పండి",
  retell_short_story:
    "చిన్న కథ విని దాని ప్రారంభం, మధ్య భాగం, ముగింపును తిరిగి చెప్పండి",
  speak_connected_sentences:
    "తెలిసిన ఒక విషయం గురించి వాక్యాలను అనుసంధానించి చెప్పడం సాధన చేయండి",
  generateSummary: "సారాంశం రూపొందించండి",
  summaryMissing: "ఈ నివేదికకు అవసరమైన సారాంశం ఇంకా రూపొందించలేదు.",
  writing_summary: "సారాంశం రూపొందుతోంది",
  slm_summary_pending: "సారాంశం సిద్ధమవుతోంది",
  planning_practice: "సాధనకు ప్రాధాన్యతలు",
  slm_disabled: "నియమాధారిత సూచనలు",
  slm_insufficient_evidence: "SLM దాటవేయబడింది · ఆధారాలు లేవు",
  slm_model_unavailable: "SLM అమర్చలేదు",
  slm_model_too_large: "SLM మోడల్ పరిమితి మించింది",
  slm_invalid_model: "SLM మోడల్ లేదు",
  slm_memory_budget:
    "సారాంశం అందుబాటులో లేదు · మెమరీ ఖాళీ చేసి మళ్లీ ప్రయత్నించండి",
  slm_context_budget: "సారాంశం అందుబాటులో లేదు · ఇన్‌పుట్ పరిమితి మించింది",
  slm_failed_or_unvalidated_output:
    "సారాంశం ధృవీకరణ విఫలమైంది · మళ్లీ ప్రయత్నించండి",
});
let ui = localStorage.getItem("ela-ui") === "te" ? "te" : "en";
const t = (key) => strings[ui][key] || strings.en[key] || key;
const number = (value, digits = 1) =>
  typeof value === "number"
    ? new Intl.NumberFormat(ui, { maximumFractionDigits: digits }).format(value)
    : "—";
let records = [],
  health = null,
  page = 1,
  activeView = "assessments",
  selectedId = null,
  pollTimer,
  reportSignature = "";
const pageSize = 15;

function node(tag, text = "", className = "") {
  const element = document.createElement(tag);
  element.textContent = text;
  if (className) element.className = className;
  return element;
}
function notify(text, error = false) {
  const element = $("#message");
  element.replaceChildren(node("span", text));
  element.hidden = false;
  element.classList.toggle("error", error);
  const close = node("button", "×");
  close.type = "button";
  close.setAttribute("aria-label", t("close"));
  close.addEventListener("click", () => (element.hidden = true));
  element.append(close);
}
async function api(path, options) {
  let response;
  try {
    response = await fetch(path, options);
  } catch {
    throw new Error(t("networkError"));
  }
  if (!response.ok) {
    const data = await response.json().catch(() => ({}));
    throw new Error(data.error || `${t("networkError")} (${response.status})`);
  }
  return response.status === 204 ? null : response.json();
}
function setNavigation(open) {
  document.body.classList.toggle("sidebar-open", open);
  $("#sidebar-scrim").hidden = !open;
  $("#menu-toggle").setAttribute("aria-expanded", String(open));
}
function route() {
  const hash = location.hash.slice(1);
  activeView = ["overview", "assessments", "students", "models"].includes(hash)
    ? hash
    : "assessments";
  for (const view of ["overview", "assessments", "students", "models"])
    $("#" + view + "-panel").hidden = view !== activeView;
  document.querySelectorAll("[data-view]").forEach((link) => {
    if (link.dataset.view === activeView)
      link.setAttribute("aria-current", "page");
    else link.removeAttribute("aria-current");
  });
  $("#page-title").textContent = t(activeView);
  $("#breadcrumb").textContent = t(activeView);
  document.title = `ELA · ${t(activeView)}`;
  $("#page-count").hidden = activeView !== "assessments";
  $("#export").hidden = activeView === "models";
  $("#new-assessment").hidden = activeView === "models";
  $("#import-collection").hidden = activeView === "models";
  setNavigation(false);
}
function applyLanguage() {
  document.documentElement.lang = ui;
  $("#ui-language").value = ui;
  document
    .querySelectorAll("[data-i18n]")
    .forEach((element) => (element.textContent = t(element.dataset.i18n)));
  $("#search").placeholder = t("search");
  route();
  updateFile();
  modelStatus();
  render();
  renderModels();
  reportSignature = "";
  renderReport();
  window.dispatchEvent(new Event("ela:language"));
}
function badge(text, tone) {
  return node("span", text, "badge " + tone);
}
function isFinished(record) {
  return ["complete", "failed"].includes(record.status);
}
function statusBadge(record) {
  return badge(
    t(record.status),
    isFinished(record) ? record.status : "processing",
  );
}
function scoreValue(record) {
  return record.report?.reading_screen?.score;
}
function dateLabel(value) {
  return new Date(value).toLocaleDateString(ui === "te" ? "te-IN" : "en-IN", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });
}
function rows(container, data, actions) {
  container.replaceChildren();
  if (!data.length) {
    const cell = node(
      "td",
      t(records.length ? "noMatch" : "empty"),
      "empty-state",
    );
    cell.colSpan = actions ? 7 : 6;
    const row = document.createElement("tr");
    row.append(cell);
    container.append(row);
    return;
  }
  for (const record of data) {
    const row = document.createElement("tr");
    const learner = document.createElement("td");
    const link = node("button", record.learner, "record-link");
    link.type = "button";
    link.addEventListener("click", () => openReport(record.id));
    learner.append(link, node("span", record.prompt, "cell-secondary"));
    row.append(learner);
    row.append(
      node("td", record.classroom),
      node("td", record.language === "te" ? "తెలుగు" : "English"),
    );
    const status = document.createElement("td");
    status.append(statusBadge(record));
    row.append(status);
    row.append(
      node(
        "td",
        scoreValue(record) == null ? "—" : `${number(scoreValue(record))}%`,
        "score-cell",
      ),
      node(
        "td",
        dateLabel(record.recorded_at || record.created_at),
        "date-cell",
      ),
    );
    if (actions) {
      const cell = document.createElement("td");
      const button = node("button", "↗", "row-action");
      button.type = "button";
      button.setAttribute(
        "aria-label",
        `${t("viewReport")} · ${record.learner}`,
      );
      button.addEventListener("click", () => openReport(record.id));
      cell.append(button);
      row.append(cell);
    }
    container.append(row);
  }
}
function filtered() {
  const query = $("#search").value.trim().toLocaleLowerCase();
  const status = $("#status-filter").value;
  const language = $("#language-filter").value;
  return records.filter(
    (record) =>
      (!query ||
        `${record.learner} ${record.classroom} ${record.prompt}`
          .toLocaleLowerCase()
          .includes(query)) &&
      (!language || record.language === language) &&
      (!status ||
        (status === "processing"
          ? !isFinished(record)
          : record.status === status)),
  );
}
function render() {
  $("#nav-count").textContent = number(records.length, 0);
  $("#page-count").textContent = number(records.length, 0);
  const data = filtered();
  const pages = Math.max(1, Math.ceil(data.length / pageSize));
  page = Math.min(page, pages);
  const start = (page - 1) * pageSize;
  rows($("#records"), data.slice(start, start + pageSize), true);
  rows($("#recent-records"), records.slice(0, 6), false);
  $("#record-range").textContent =
    `${data.length ? start + 1 : 0}–${Math.min(start + pageSize, data.length)} ${t("of")} ${data.length}`;
  $("#page-number").textContent = `${page} / ${pages}`;
  $("#previous").disabled = page <= 1;
  $("#next").disabled = page >= pages;
  const stats = $("#stats");
  stats.replaceChildren();
  const values = [
    ["total", records.length],
    [
      "learners",
      new Set(
        records
          .filter(
            (record) =>
              record.student_id &&
              !record.student_id.startsWith("legacy-assessment-"),
          )
          .map((record) => record.student_id),
      ).size,
    ],
    ["processing", records.filter((record) => !isFinished(record)).length],
    [
      "needsSupport",
      records.filter(
        (record) => record.report?.reading_screen?.level === "needs_support",
      ).length,
    ],
  ];
  for (const [label, value] of values) {
    const stat = node("div", "", "stat");
    stat.append(
      node("div", t(label), "stat-label"),
      node("div", number(value, 0), "stat-value"),
    );
    stats.append(stat);
  }
}
function toggleReference() {
  const reading = $("#task").value === "read_aloud";
  const photo = $("#reference-source").value === "photo";
  $("#reference-source-field").hidden = !reading;
  $("#reference-field").hidden = !reading || photo;
  $("#reference").required = reading && !photo;
  $("#paragraph-photo-field").hidden = !reading || !photo;
  $("#paragraph-photo").required = reading && photo;
}
function updateFile() {
  const file = $("#audio").files[0];
  $("#file-label").textContent = file ? file.name : t("dropAudio");
  $("#file-size").textContent = file
    ? `${number(file.size / 1024 / 1024)} MB`
    : "64 MB";
}
function modelStatus() {
  const conversation = $("#audio-mode").value === "conversation";
  $("#student-reference-field").hidden = !conversation;
  $("#student-reference").required = conversation;
  const current = health?.languages[$("#assessment-language").value];
  const ready =
    current?.configured &&
    (!conversation || health?.diarization_configured) &&
    ($("#detect-language").value !== "true" ||
      health?.language_switching_configured) &&
    ($("#task").value !== "read_aloud" ||
      $("#reference-source").value !== "photo" ||
      health?.ocr_configured);
  $("#model-status").textContent = t(
    !health ? "checking" : ready ? "modelReady" : "modelMissing",
  );
  $("#model-dot").classList.toggle("offline", !ready);
}
function renderModels() {
  const cards = $("#model-cards");
  cards.replaceChildren();
  for (const language of ["te", "en"]) {
    const model = health?.languages[language];
    const card = node("article", "", "panel model-card");
    const header = document.createElement("header");
    header.append(
      node("h2", language === "te" ? "తెలుగు" : "English"),
      badge(
        t(
          !health
            ? "checking"
            : model?.configured
              ? "configured"
              : "notConfigured",
        ),
        model?.configured ? "complete" : "failed",
      ),
    );
    card.append(header);
    const list = document.createElement("dl");
    list.append(
      node("dt", t("backend")),
      node(
        "dd",
        t(model?.backend === "external-local-adapter" ? "external" : "native"),
      ),
      node("dt", t("configuration")),
    );
    const variables =
      language === "te"
        ? ["ELA_TE_MODEL", "ELA_TE_TOKENS"]
        : ["ELA_EN_ENCODER", "ELA_EN_DECODER", "ELA_EN_TOKENS"];
    for (const variable of variables) list.append(node("dd", variable));
    list.append(
      node("dt", t("converter")),
      node(
        "dd",
        !health
          ? "…"
          : health.conversion_available
            ? "FFmpeg · " + t("available")
            : t("unavailable"),
      ),
    );
    card.append(list);
    cards.append(card);
  }
  for (const [label, configured, variables] of [
    [
      "diarization",
      health?.diarization_configured,
      ["ELA_DIARIZATION_MODEL", "ELA_SPEAKER_MODEL"],
    ],
    [
      "languageDetection",
      health?.language_switching_configured,
      ["ELA_LID_MODEL"],
    ],
    ["paragraphPhoto", health?.ocr_configured, ["Tesseract · tel + eng"]],
    ["nlpTelugu", health?.nlp?.te, ["ELA_NLP_TE_MODEL"]],
    [
      "nlpEnglish",
      health?.nlp?.en,
      health?.english_spacy_configured
        ? ["ELA_SPACY_MODEL", "ELA_NLP_PYTHON"]
        : ["ELA_NLP_EN_MODEL"],
    ],
    [
      "grammarModel",
      health?.english_grammar_configured,
      ["ELA_T5_MODEL_DIR", "ELA_SPACY_MODEL", "ELA_NLP_PYTHON"],
    ],
    [
      "practiceSlm",
      health?.slm_configured,
      ["ELA_SLM_MODEL", "ELA_LLAMA_BIN (llama-completion)"],
    ],
  ]) {
    const card = node("article", "", "panel model-card");
    const header = document.createElement("header");
    header.append(
      node("h2", t(label)),
      badge(
        t(!health ? "checking" : configured ? "configured" : "notConfigured"),
        configured ? "complete" : "failed",
      ),
    );
    const list = document.createElement("dl");
    list.append(node("dt", t("configuration")));
    for (const variable of variables) list.append(node("dd", variable));
    card.append(header, list);
    cards.append(card);
  }
}
async function refresh() {
  clearTimeout(pollTimer);
  try {
    records = await api("/api/assessments");
    window.dispatchEvent(new Event("ela:refresh"));
    render();
    renderReport();
    $("#connection-dot").classList.remove("offline");
    $("#connection-label").textContent = t("local");
    pollTimer = setTimeout(
      refresh,
      records.some((record) => !isFinished(record)) ? 1500 : 12000,
    );
  } catch (error) {
    $("#connection-dot").classList.add("offline");
    $("#connection-label").textContent = t("offline");
    pollTimer = setTimeout(refresh, 5000);
  }
}
function openReport(id) {
  selectedId = id;
  reportSignature = "";
  renderReport();
  $("#report-dialog").showModal();
}
function metric(container, label, value) {
  const item = node("div", "", "metric");
  item.append(node("dt", t(label)), node("dd", number(value)));
  container.append(item);
}
function codeList(container, title, codes) {
  const section = document.createElement("section");
  section.append(node("h3", t(title)));
  const list = document.createElement("ul");
  for (const code of codes) list.append(node("li", t(code)));
  section.append(list);
  container.append(section);
}
function renderReport() {
  const record = records.find((item) => item.id === selectedId);
  if (!record) return;
  const signature = JSON.stringify(record) + ui;
  if (signature === reportSignature) return;
  reportSignature = signature;
  $("#report-title").textContent = record.learner;
  $("#report-meta").textContent =
    `${record.classroom} · ${record.language === "te" ? "తెలుగు" : "English"} · ${dateLabel(record.recorded_at || record.created_at)}`;
  $("#delete-report").disabled = !isFinished(record);
  const content = $("#report-content");
  const diagnosticsOpen = content.querySelector(
    "[data-main-diagnostics]",
  )?.open;
  content.replaceChildren();
  if (record.error) {
    content.append(node("p", record.error, "danger"));
    if (!record.report) return;
  }
  if (!record.report) {
    content.append(statusBadge(record));
    return;
  }
  const report = record.report,
    screen = report.reading_screen;
  const hero = node("div", "", "report-hero");
  if (screen) {
    const score = node("div", number(screen.score), "report-score");
    score.append(node("small", "/ 100"));
    const labels = document.createElement("div");
    labels.append(
      badge(t(screen.level), screen.level),
      badge(t("provisional"), "provisional"),
    );
    hero.append(score, labels);
  } else
    hero.append(
      badge(
        t(report.outcome),
        report.outcome === "measured" ? "complete" : "failed",
      ),
    );
  content.append(hero);
  content.append(node("h3", t("summary")));
  if (report.slm?.summary) {
    const explanation = node(
      "p",
      report.slm.summary[ui] || report.slm.summary.en,
      "transcript",
    );
    explanation.lang = ui;
    content.append(explanation);
  } else content.append(node("p", t("summaryMissing"), "muted"));
  if (!isFinished(record)) content.append(statusBadge(record));
  else if (!report.slm?.summary || record.error) {
    const regenerate = node("button", t("generateSummary"));
    regenerate.type = "button";
    regenerate.addEventListener("click", async () => {
      regenerate.disabled = true;
      try {
        await api(`/api/assessments/${record.id}/summary`, { method: "POST" });
        await refresh();
      } catch (error) {
        notify(error.message, true);
        regenerate.disabled = false;
      }
    });
    content.append(regenerate);
  }
  if (isFinished(record)) {
    const update = node("button", t("updateLinguistics"));
    update.type = "button";
    update.addEventListener("click", async () => {
      update.disabled = true;
      try {
        await api(`/api/assessments/${record.id}/linguistics`, {
          method: "POST",
        });
        await refresh();
      } catch (error) {
        notify(error.message, true);
        update.disabled = false;
      }
    });
    content.append(update);
  }
  const metrics = node("dl", "", "metrics");
  metric(metrics, "recognized", report.word_count);
  metric(
    metrics,
    screen ? "pace" : "observedPace",
    screen?.matched_words_per_minute ?? report.words_per_recording_minute,
  );
  metric(metrics, "duration", report.audio.duration_seconds);
  metric(metrics, "wer", report.reference_comparison?.word_error_rate_percent);
  content.append(metrics);
  content.append(node("h3", t("transcript")));
  const transcript = node("p", report.transcript || "—", "transcript");
  transcript.lang = record.language;
  transcript.dir = "auto";
  content.append(transcript);
  if (screen?.practice_words.length) {
    content.append(node("h3", t("focusWords")));
    const chips = node("div", "", "word-chips");
    for (const word of screen.practice_words)
      chips.append(node("span", word, "word-chip"));
    content.append(chips);
  }
  if (report.slm?.summary && report.slm?.plan) {
    const practice = node("div", "", "practice-grid");
    codeList(practice, "practice", report.slm.plan.practice);
    if (report.slm.plan.interventions.length)
      codeList(practice, "intervention", report.slm.plan.interventions);
    content.append(practice);
  }
  if (report.grammar && report.grammar.status !== "not_applicable") {
    const grammar = report.grammar;
    content.append(
      node("h3", t("grammarTitle")),
      node("p", t("grammarNotice"), "muted"),
    );
    if (grammar.task_context === "read_aloud")
      content.append(node("p", t("grammarReadAloud"), "muted"));
    content.append(
      node(
        "p",
        `${t("grammarReviewed")}: ${grammar.analyzed_sentences} / ${grammar.total_sentences}`,
        "muted",
      ),
    );
    if (grammar.status === "partial")
      content.append(badge(t("grammarPartial"), "provisional"));
    if (!["complete", "partial"].includes(grammar.status))
      content.append(
        badge(
          t(
            grammar.status === "memory_blocked"
              ? "grammarMemoryBudget"
              : "grammarUnavailable",
          ),
          "failed",
        ),
      );
    else if (!Object.keys(grammar.category_counts).length)
      content.append(node("p", t("grammarNoChanges")));
    for (const correction of grammar.corrections.filter(
      (c) =>
        c.status === "complete" && c.original !== c.suggested && c.edits.length,
    )) {
      const section = node("section", "", "panel grammar-correction");
      section.append(
        node("h4", t("grammarOriginal")),
        node("p", correction.original, "transcript"),
        node("h4", t("grammarSuggested")),
        node("p", correction.suggested, "transcript"),
      );
      const edits = node("ul");
      for (const edit of correction.edits) {
        const key = edit.category.includes("VERB:SVA")
          ? "grammarSva"
          : edit.category.includes("VERB:TENSE")
            ? "grammarTense"
            : edit.category.includes("VERB:FORM")
              ? "grammarVerbForm"
              : edit.category.includes("DET")
                ? "grammarDeterminer"
                : edit.category.includes("NOUN:NUM")
                  ? "grammarNumber"
                  : edit.category.includes("PREP")
                    ? "grammarPreposition"
                    : edit.category.includes("WO")
                      ? "grammarOrder"
                      : "grammarOther";
        edits.append(
          node(
            "li",
            `${t(key)}: ${edit.original || "∅"} → ${edit.suggested || "∅"}`,
          ),
        );
      }
      section.append(edits);
      content.append(section);
    }
  }
  if (report.nlp) {
    const languageDetails = node("details", "", "report-diagnostics");
    languageDetails.append(node("summary", t("nlpAnalysis")));
    content.append(languageDetails);
    if (!report.nlp.units.length)
      languageDetails.append(badge(t("nlpUnavailable"), "failed"));
    for (const unit of report.nlp.units) {
      languageDetails.append(
        node("p", unit.language === "te" ? "తెలుగు" : "English", "muted"),
      );
      const languageMetrics = node("dl", "", "metrics");
      metric(languageMetrics, "lexicalDensity", unit.lexical_density_percent);
      metric(languageMetrics, "lemmas", unit.unique_lemmas);
      metric(languageMetrics, "sentences", unit.sentences);
      metric(
        languageMetrics,
        "dependencyDistance",
        unit.mean_dependency_distance,
      );
      languageDetails.append(languageMetrics);
      const tags = node("div", "", "word-chips");
      for (const [tag, count] of Object.entries(unit.pos_counts))
        tags.append(node("span", `${tag} · ${count}`, "word-chip"));
      languageDetails.append(tags);
      const linguisticDetails = document.createElement("details");
      linguisticDetails.className = "report-diagnostics";
      linguisticDetails.append(node("summary", t("morphology")));
      for (const [feature, values] of Object.entries(
        unit.morphological_features,
      ))
        linguisticDetails.append(
          node(
            "p",
            `${feature}: ${Object.entries(values)
              .map(([key, value]) => `${key} (${value})`)
              .join(", ")}`,
          ),
        );
      linguisticDetails.append(
        node(
          "p",
          Object.entries(unit.dependency_counts)
            .map(([key, value]) => `${key} (${value})`)
            .join(", "),
        ),
      );
      languageDetails.append(linguisticDetails);
    }
    if (report.nlp.reference_content_lemma_coverage_percent != null) {
      const coverage = node("dl", "", "metrics");
      metric(
        coverage,
        "lemmaCoverage",
        report.nlp.reference_content_lemma_coverage_percent,
      );
      languageDetails.append(coverage);
    }
  }
  const details = node("details", "", "report-diagnostics");
  details.dataset.mainDiagnostics = "true";
  details.open = Boolean(diagnosticsOpen);
  if (report.ocr)
    details.append(
      node(
        "p",
        `${t("ocrConfidence")}: ${number(report.ocr.mean_word_confidence)}`,
      ),
    );
  if (report.nlp) codeList(details, "nlpAnalysis", report.nlp.warnings);
  details.append(node("summary", t("diagnostics")));
  codeList(details, "status", report.warnings);
  if (report.speech) {
    for (const [label, value] of [
      ["speakers", report.speech.speaker_count],
      ["studentSeconds", report.speech.student_seconds],
      ["voiceSimilarity", report.speech.speaker_similarity],
      ["overlapSeconds", report.speech.overlap_excluded_seconds],
      ["unknownSeconds", report.speech.untranscribed_student_seconds],
    ]) {
      details.append(node("p", `${t(label)}: ${number(value)}`));
    }
  }
  if (report.segments?.length) {
    const timeline = document.createElement("details");
    timeline.className = "report-diagnostics";
    timeline.append(node("summary", t("speechSegments")));
    const table = document.createElement("table"),
      body = document.createElement("tbody");
    for (const segment of report.segments) {
      const row = document.createElement("tr");
      row.append(
        node(
          "td",
          `${number(segment.start_seconds)}–${number(segment.end_seconds)}s`,
        ),
        node(
          "td",
          segment.language === "te"
            ? "తెలుగు"
            : segment.language === "en"
              ? "English"
              : segment.language,
        ),
        node(
          "td",
          segment.exclusion_reason ? t(segment.exclusion_reason) : segment.text,
        ),
      );
      body.append(row);
    }
    table.append(body);
    timeline.append(table);
    content.append(timeline);
  }
  if (report.slm)
    details.append(
      node(
        "p",
        `${t("practiceSlm")}: ${report.slm.reason ? t(report.slm.reason) : report.slm.status}`,
      ),
    );
  if (report.nlp) details.append(node("p", report.nlp.backend));
  if (report.grammar) {
    details.append(node("p", report.grammar.backend));
    if (report.grammar.model_revision)
      details.append(node("p", report.grammar.model_revision));
    codeList(details, "grammarTitle", report.grammar.warnings);
  }
  if (report.resource_policy) details.append(node("p", report.resource_policy));
  details.append(node("p", `${t("backend")}: ${report.backend}`));
  if (report.model_revision) details.append(node("p", report.model_revision));
  if (record.reference_text) {
    details.append(
      node("h3", t("passage")),
      node("p", record.reference_text, "transcript"),
    );
  }
  content.append(details);
}
function download(data, filename) {
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(data, null, 2)], { type: "application/json" }),
  );
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
$("#ui-language").addEventListener("change", (event) => {
  ui = event.target.value;
  localStorage.setItem("ela-ui", ui);
  applyLanguage();
});
window.addEventListener("hashchange", route);
$("#menu-toggle").addEventListener("click", () =>
  setNavigation(!document.body.classList.contains("sidebar-open")),
);
$("#sidebar-scrim").addEventListener("click", () => setNavigation(false));
$("#new-assessment").addEventListener("click", () => {
  $("#form-error").hidden = true;
  $("#assessment-dialog").showModal();
});
document
  .querySelectorAll("[data-close]")
  .forEach((button) =>
    button.addEventListener("click", () =>
      $("#" + button.dataset.close).close(),
    ),
  );
for (const selector of ["#search", "#status-filter", "#language-filter"])
  $(selector).addEventListener(
    selector === "#search" ? "input" : "change",
    () => {
      page = 1;
      render();
    },
  );
$("#previous").addEventListener("click", () => {
  page--;
  render();
});
$("#next").addEventListener("click", () => {
  page++;
  render();
});
$("#task").addEventListener("change", () => {
  toggleReference();
  modelStatus();
});
$("#reference-source").addEventListener("change", () => {
  toggleReference();
  modelStatus();
});
$("#assessment-language").addEventListener("change", modelStatus);
$("#audio-mode").addEventListener("change", modelStatus);
$("#detect-language").addEventListener("change", modelStatus);
$("#audio").addEventListener("change", updateFile);
const dropzone = $("#dropzone");
for (const name of ["dragenter", "dragover"])
  dropzone.addEventListener(name, (event) => {
    event.preventDefault();
    dropzone.classList.add("drag-over");
  });
for (const name of ["dragleave", "drop"])
  dropzone.addEventListener(name, (event) => {
    event.preventDefault();
    dropzone.classList.remove("drag-over");
  });
dropzone.addEventListener("drop", (event) => {
  if (event.dataTransfer.files.length) {
    const transfer = new DataTransfer();
    transfer.items.add(event.dataTransfer.files[0]);
    $("#audio").files = transfer.files;
    updateFile();
  }
});
$("#assessment-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const form = event.target;
  const file = $("#audio").files[0];
  const error = $("#form-error");
  error.hidden = true;
  const referenceBytes =
    $("#audio-mode").value === "conversation"
      ? $("#student-reference").files[0]?.size || 0
      : 0;
  const photoBytes =
    $("#task").value === "read_aloud" &&
    $("#reference-source").value === "photo"
      ? $("#paragraph-photo").files[0]?.size || 0
      : 0;
  if (
    (file?.size || 0) + referenceBytes + photoBytes >
    64 * 1024 * 1024 - 65536
  ) {
    error.textContent = t("tooLarge");
    error.hidden = false;
    return;
  }
  const data = new FormData(form);
  if ($("#audio-mode").value !== "conversation")
    data.delete("student_reference");
  data.set("recorded_at", new Date().toISOString());
  if (
    $("#task").value !== "read_aloud" ||
    $("#reference-source").value === "photo"
  )
    data.set("reference_text", "");
  if (
    $("#task").value !== "read_aloud" ||
    $("#reference-source").value !== "photo"
  )
    data.delete("paragraph_photo");
  $("#assess").disabled = true;
  try {
    const result = await api("/api/assessments", {
      method: "POST",
      body: data,
    });
    $("#assessment-dialog").close();
    form.reset();
    toggleReference();
    updateFile();
    modelStatus();
    location.hash = "assessments";
    page = 1;
    notify(t("submitted"));
    await refresh();
    openReport(result.id);
  } catch (failure) {
    error.textContent = failure.message;
    error.hidden = false;
  } finally {
    $("#assess").disabled = false;
  }
});
$("#delete-report").addEventListener("click", async () => {
  if (!confirm(t("deleteConfirm"))) return;
  try {
    await api(`/api/assessments/${selectedId}`, { method: "DELETE" });
    $("#report-dialog").close();
    selectedId = null;
    notify(t("deleted"));
    refresh();
  } catch (error) {
    notify(error.message, true);
  }
});
$("#export-report").addEventListener("click", () => {
  const record = records.find((item) => item.id === selectedId);
  if (record) download(record, `ela-assessment-${record.id}.json`);
});
$("#export").addEventListener("click", async () => {
  try {
    download(await api("/api/assessments"), "ela-assessments.json");
  } catch (error) {
    notify(error.message, true);
  }
});
$("#export-legacy").addEventListener("click", async () => {
  try {
    download(
      await api("/api/legacy-assessments"),
      "ela-previous-assessments.json",
    );
  } catch (error) {
    notify(error.message, true);
  }
});
applyLanguage();
toggleReference();
refresh();
api("/api/health")
  .then((data) => {
    health = data;
    modelStatus();
    renderModels();
  })
  .catch(() => {});
