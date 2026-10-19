# Launcher speech follow-up (VOICE-001)

> Retained design record. Unchecked historical milestones are not a current release checklist. See docs/release-status.md for current scope and outstanding validation.


Status: deferred, not implemented for the launcher chat release.
Existing web/mobile/provider speech contracts and implementations stay intact.

The helper command `voice` must return a structured `not_implemented` result with
specId VOICE-001, not panic, start recording, or claim microphone support. UI must
hide/disable unsupported voice controls rather than display a working microphone.

Host adapters own permission, capture readiness, VAD/endpointing and playback.
Core owns explicit turn state and cancellation. Provider adapters support
OpenAI-compatible transcription (/v1/audio/transcriptions) and synthesis
(/v1/audio/speech), model/voice/format discovery and connection tests.
Device dictation/TTS are optional host-specific alternatives, not assumed parity.

Acceptance: no clipped initial speech, automatic endpointing, explicit cancel,
interruption drops pending audio, no actions from partial transcripts, text fallback,
denied permission/offline recovery, latency measurements, ephemeral audio cleanup.
Never copy saved clip libraries or public environment API keys from other apps.
Each host's microphone and interruption support requires native proof.
