# Application Web Parole-à-Parole Material Design 3

Il s'agit d'une application web d'assistant vocal IA locale, en temps réel et parole-à-parole, qui s'exécute entièrement hors ligne sans nécessiter de clés API externes. Elle utilise le SDK Python `google-antigravity` avec :
- **Reconnaissance vocale (STT) :** Modèle Whisper local basé sur CTranslate2 (`faster-whisper`).
- **Synthèse vocale (TTS) :** Modèle Kokoro local au format ONNX (`kokoro-onnx`).
- **Orchestration :** Agent standard Antigravity avec `LocalConnectionStrategy`.
- **Interface utilisateur :** Une interface web inspirée de Google Material Design 3 et de la marque Gemini, utilisant du CSS natif, une gemme de visualisation pulsante et une détection d'activité vocale (VAD) locale dans le navigateur.

---

## Architecture

```
                       [ Navigateur Web ]
                                |
             (Connexion WebSocket ws://localhost:8789)
                                |
                                v
                     [ server.py (Backend) ]
         _______________________|_______________________
        |                       |                       |
        v                       v                       v
 [ Whisper STT Local ]    [ Agent Antigravity ]   [ Kokoro TTS Local ]
```

1. **Capture du microphone :** Le navigateur capture l'entrée micro en 16kHz mono.
2. **VAD côté client :** La surveillance du niveau RMS en temps réel dans JavaScript détecte les limites de la parole.
3. **Flux audio :** Les trames PCM float32 binaires sont envoyées via WebSockets pendant que l'utilisateur parle.
4. **Transcription Whisper :** Après un délai de silence, le serveur transcrit la parole avec Whisper.
5. **Exécution de l'agent :** La transcription est envoyée à l'agent local qui renvoie un flux de réponse texte.
6. **Synthèse concurrente :** Les segments de texte sont envoyés à Kokoro TTS dès qu'une ponctuation (y compris virgules et points-virgules) est rencontrée.
7. **Lecture audio fluide :** Les trames PCM float32 24kHz synthétisées sont renvoyées au client et planifiées pour une lecture sans coupure via AudioContext.
8. **Interruption active (Barge-in) :** Si l'utilisateur commence à parler pendant que l'agent s'exprime, la génération et la lecture de la réponse en cours sont immédiatement interrompues.

---

## Configuration et Installation

### 1. Configurer l'environnement Python

Assurez-vous d'avoir installé les dépendances `voice` du paquet `google-antigravity`.

Avec `uv` :
```bash
# Dans le dossier python/antigravity-sdk-python
uv pip install -e ".[voice]"
uv pip install websockets
```

### 2. Lancer le serveur WebSocket

Démarrez le serveur avec `server.py` :
```bash
python server.py
```

*Note : Lors du premier lancement, le serveur téléchargera automatiquement le modèle Kokoro ONNX par défaut (`kokoro-v0_19.onnx`, ~80 Mo) et la configuration des voix (`voices.json`, ~20 Mo) depuis Hugging Face dans un dossier local `./models/`.*

Vous pouvez personnaliser les paramètres via la ligne de commande :
```bash
python server.py --host 127.0.0.1 --port 8789 --whisper-model tiny --voice-name ff_siwis
```

---

## Lancement du Frontend Web

Le frontend étant construit avec des API HTML5 standards et du CSS natif, vous pouvez l'ouvrir directement dans votre navigateur :

1. Double-cliquez sur [index.html](index.html) ou lancez un serveur web local simple :
   ```bash
   # Avec Python
   python -m http.server 8000
   # Avec Bun
   bunx serve
   ```
2. Ouvrez `http://localhost:8000` (ou le chemin du fichier local) dans Chrome, Edge ou Firefox.
3. Cliquez sur la gemme de visualisation lumineuse au centre de l'écran pour vous connecter au serveur WebSocket.
4. Autorisez l'accès au microphone lorsque cela vous est demandé.
5. Parlez ! Le visualiseur changera de taille et pulsera, et le journal affichera les transcriptions en temps réel.

---

## Optimisation de la latence

Pour obtenir une réactivité maximale et une latence minimale :
1. **Modèle Whisper plus petit :** Utilisez le paramètre `--whisper-model tiny` (ou `base`) lors du lancement du serveur. Les modèles plus petits s'exécutent beaucoup plus rapidement sur CPU.
2. **Délai de silence court :** Réglez le curseur de délai de silence sur `0.4s` ou `0.3s` dans l'interface des paramètres. Cela permet de déclencher la transcription dès que vous arrêtez de parler.
3. **Synthèse sur virgules/points-virgules :** Le serveur commence la génération audio Kokoro dès qu'il rencontre une virgule ou un point-virgule, permettant de commencer la lecture audio avant même que la phrase entière ne soit générée par l'agent.

---

## Intégration du Design System

Le design applique les spécifications du fichier [DESIGN.md](../../../DESIGN.md) du dépôt parent :
- **Typographie :** Utilise Outfit / Google Sans Flex pour des graisses typographiques adaptables.
- **Palette de couleurs :** Respecte strictement les valeurs de base M3 (couleur primaire violette, nuances des conteneurs) et le dégradé de la marque Gemini (bleu -> violet -> rose).
- **Mise en page :** Un panneau latéral moderne pour la configuration du seuil, une arène centrale pour les signaux vocaux et un tiroir rétractable pour l'historique des conversations.
