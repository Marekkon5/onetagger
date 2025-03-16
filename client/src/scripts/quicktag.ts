import { toRaw } from "vue";
import { QuickTagSettings } from "./settings";
import { FrameName, Keybind } from "./utils";
import { get1t } from "./onetagger";

class QuickTag { 
    tracks: QTTrack[] = [];
    track: QTMultiTrack = new QTMultiTrack();
    failed: QuickTagFailed[] = [];
    wasLimited = false;

    /// Number of tracks to save
    saving: number = 0;

    // Track to load in tag editor
    toTagEditor?: string;

    constructor() {}

    /// Wait for saving to finish
    async waitForSave() {
        while (this.saving > 0) {
            await new Promise((r, _) => setTimeout(() => r(null), 10));
        }
    }

    /// Check if the loading was limited
    isLimited() {
        return (this.wasLimited && (this.tracks.length + this.failed.length) == 500);
    }
}

interface QuickTagFailed {
    path: string,
    error: string,
}

interface QuickTagMood {
    mood: string;
    color: string,
    keybind?: Keybind
    outline?: boolean;
}

class EnergyTag {
    type: 'rating' | 'symbol' = 'rating';
    symbol: string = '*';
    tag: FrameName = new FrameName('TCOM', 'COMPOSER', '©wrt')

    static fromJson(data: any): EnergyTag {
        let et: EnergyTag = Object.assign(new EnergyTag(), data);
        et.tag = FrameName.fromJson(data.tag);
        return et;
    }
}

interface QuickTagGenre {
    genre: string;
    keybind?: Keybind;
    subgenres: string[]
}

interface QuickTagCustom {
    name: string;
    tag: FrameName;
    values: QuickTagCustomValue[];
}

interface QuickTagCustomValue {
    val: string;
    keybind?: Keybind;
}

interface QuickTagFile {
    path: string;
    format: 'flac' | 'aiff' | 'mp3' | 'mp4' | 'wav' | 'ogg';
    title: string;
    artists: string[];
    genres: string[];
    bpm?: number;
    rating: number;
    tags: Record<string, string[]>;
    year?: number;
    key?: string;
}

/// Custom Tag chips
interface CustomTagInfo {
    value: string,
    type: 'custom' | 'note';
    index: number
}


/// Replacement for QTTrack to support multiple tracks at once
class QTMultiTrack {

    // Selected tracks
    tracks: QTTrack[] = [];

    constructor() {}

    /// Add new track
    addTrack(track: QTTrack) {
        if (this.tracks.find(t => t.path == track.path)) return;
        this.tracks.push(track);
    }

    /// Remove track from the multitrack
    removeTrack(track: QTTrack) {
        let i = this.tracks.findIndex(t => t.path == track.path);
        if (i != -1) {
            this.tracks.splice(i, 1);
        }
    }

    /// Remove all tracks
    removeAll() {
        this.tracks.length = 0;
    }

    /// Get selected track by path
    getTrack(path: string): QTTrack | undefined {
        return this.tracks.find(t => t.path == path);
    }

    /// Whether are tracks selected
    hasTracks(): boolean {
        return this.tracks.length > 0;
    }

    /// Remove all tracks and replace with single one
    loadSingle(track: QTTrack) {
        this.tracks.length = 0;
        this.tracks.push(track);
    }

    /// Is the track selected
    isSelected(track: QTTrack): boolean {
        return this.tracks.find(t => t.path == track.path) !== undefined;
    }

    /// Get mood of selected tracks (set / get for compatibility)
    get mood(): string | undefined {
        if (this.tracks.length == 0) return;
        // All tracks have same mood
        let mood = this.tracks[0].mood;
        if (this.tracks.every(t => t.mood == mood)) return mood;
        // Different moods = no mood selected
        return;
    }

    /// Set the mood in selected tracks, (setter for compatibility)
    set mood(mood: string | undefined) {
        for (let i=0; i<this.tracks.length; i++) {
            this.tracks[i].mood = mood;
        }
    }

    /// Get energy of selected tracks (set / get for compatibility)
    get energy(): number {
        if (this.tracks.length == 0) return 0;
        // All tracks have same energy
        let energy = this.tracks[0].energy;
        if (this.tracks.every(t => t.energy == energy)) return energy;
        // Different energies = 0
        return 0;
    }

    /// Set the energy of selected tracks (set / get for compatibility)
    set energy(energy: number) {
        for (let i=0; i<this.tracks.length; i++) {
            this.tracks[i].energy = energy;
        }
    }

    /// Get genres present in all songs (get for compatibility)
    get genres(): string[] {
        if (this.tracks.length == 0) return [];
        return this.tracks[0].genres.filter(g => this.tracks.every(t => t.genres.includes(g)));
    }

    /// Get note (if all tracks have the same one)
    getNote(): string {
        if (this.tracks.length == 0) return '';
        let note = this.tracks[0].note;
        if (this.tracks.every(t => t.note == note)) return note;
        return '';
    }

    /// Set note on all tracks
    setNote(note: string) {
        for (let i=0; i<this.tracks.length; i++) {
            this.tracks[i].setNote(note); // Use the sanitizing setNote method
        }
    }

    /// Toggle genere on all selected tracks
    toggleGenre(genre: string) {
        // Remove genre
        if (this.tracks.find(t => t.genres.includes(genre))) {
            for (let i=0; i<this.tracks.length; i++) {
                let j = this.tracks[i].genres.indexOf(genre);
                if (j != -1) {
                    this.tracks[i].genres.splice(j, 1);
                }
            }
            return;
        }
        // Add genre
        for (let i=0; i<this.tracks.length; i++) {
            if (!this.tracks[i].genres.includes(genre)) {
                this.tracks[i].genres.push(genre);
            }
        }
    }

    /// Check if custom tag is selected
    getCustom(tag: number, value: string) {
        if (this.tracks.length == 0) return false;
        return this.tracks.every(t => (t.custom[tag]??[]).includes(value));
    }

    /// Sort custom tags according to settings
    sortCustom() {
        for (let i=0; i<this.tracks.length; i++) {
            for (let j=0; j<this.tracks[i].custom.length; j++) {
                this.tracks[i].sortCustom(j);
            }
        }
    }

    /// Toggle custom value on all tracks
    toggleCustom(tag: number, value: string) {
        // New value
        for (let i=0; i<this.tracks.length; i++) {
            if (!this.tracks[i].custom[tag]) this.tracks[i].custom[tag] = [];
        }

        // Remove existing
        if (this.tracks.find(t => t.custom[tag].includes(value))) {
            for (let i=0; i<this.tracks.length; i++) {
                this.tracks[i].removeCustom(tag, value);
            }
            return;
        }

        // Add new
        for (let i=0; i<this.tracks.length; i++) {
            this.tracks[i].addCustom(tag, value);
        }
    }

    /// Is any of the tracks changed
    isChanged(): boolean {
        return this.tracks.find(t => t.isChanged()) !== undefined;
    }

    /// Get all tracks changes
    getOutputs(): object[] {
        let output = [];
        for (let i=0; i<this.tracks.length; i++) {
            let out = this.tracks[i].getOutput();
            if (out.changes.length > 0) {
                output.push(out);
            }
        }
        return output;
    }
}

class QTTrack implements QuickTagFile {
    // QuickTagFile
    path!: string;
    format!: "flac" | "aiff" | "mp3" | "mp4" | "wav" | "ogg";
    title!: string;
    artists!: string[];
    genres!: string[];
    bpm?: number | undefined;
    rating!: number;
    tags!: Record<string, string[]>;
    year?: number | undefined;
    key?: string | undefined;

    // QTTrack
    mood?: string;
    energy: number = 0;
    note: string;
    originalNote: string;
    custom: string[][] = [];
    originalGenres: string[];

    settings: QuickTagSettings;

    constructor(data: QuickTagFile, settings: QuickTagSettings) {
        // Data from backend
        Object.assign(this, data);
        this.settings = settings;
        this.genres = this.processGenreArray(this.genres);
        this.mood = this.getMood();
        this.energy = this.getEnergy();
        this.note = this.getNote();
        this.originalNote = this.note;
        this.custom = this.loadCustom();
        // Stupid copy
        this.originalGenres = JSON.parse(JSON.stringify(this.genres));
        // Add subgenres
        if (settings.subgenreTag) {
            let subgenres = (this.tags[this.removeAbstractions(settings.subgenreTag.byFormat(this.format))] ?? []);
            this.originalGenres.push(...subgenres.filter(g => !this.genres.includes(g)));
            this.genres.push(...subgenres.filter(g => !this.genres.includes(g)));
        }
    } 
    
    // Process genres
    processGenreArray(genres: string[]): string[] {
        return genres
            .map(g => g.trim())
            .filter(g => g.length > 0);
    }

    // Check if genre was changed
    isGenreChanged(): boolean {
        // Process both arrays for consistent comparison
        const currentGenres = [...this.genres]
            .map(g => g.trim())
            .filter(g => g.length > 0)
            .sort()
            .join(',');
        const originalGenres = [...this.originalGenres]
            .map(g => g.trim())
            .filter(g => g.length > 0)
            .sort()
            .join(',');
        return currentGenres !== originalGenres;
    }

    // Process note string
    processNoteString(noteStr: string | undefined): string[] {
        if (!noteStr) return [];
        return noteStr.split(',')
            .map(n => n.trim())
            .filter(n => n.length > 0);
    }

    // Remove field name abstractions
    removeAbstractions(input: string): string {
        if (this.format != 'mp4' || !input) return input;
        // Leading
        input = input.replace('----:', '');
        // iTunes:VALUE -> com.apple.Itunes:VALUE
        if (input.startsWith('iTunes:')) input = 'com.apple.' + input;
        return input;
    }
    
    // Get note from tags
    getNote() {
        if (this.note || this.note === '') {
            return this.note;
        }
        let field = this.removeAbstractions(this.settings.noteTag.tag.byFormat(this.format));
        let noteValues = this.tags[field] ?? [];
        
        // Remove custom tags from note
        for (let custom of this.settings.custom) {
            if (custom.tag.byFormat(this.format) == field) {
                noteValues = noteValues.filter(v => !custom.values.map(i => i.val).includes(v));
            }
        }
        
        // Process and join the note values
        return this.processNoteString(noteValues.join(',')).join(',');
    }

    // Update note field
    setNote(note: string) {
        const processedParts = this.processNoteString(note);
        this.note = processedParts.join(',');
    }

    // Get mood tag value
    getMood() {
        let field = this.removeAbstractions(this.settings.moodTag.byFormat(this.format));
        if (this.tags[field]??[].length >= 1) {
            return this.tags[field][0]
        }
    }

    // Get energy value
    getEnergy() {
        // Use rating as energy
        if (this.settings.energyTag.type == 'rating') {
            return this.rating??0;
        }
        // Use custom symbols as energy
        let t = this.tags[this.removeAbstractions(this.settings.energyTag.tag.byFormat(this.format))];
        if (t) {
            // Use first element of array
            let val = '';
            if (typeof t == 'object') {
                if (t.length == 0) return 0;
                val = t[0];
            } else {
                val = t;
            }

            return val.split(this.settings.energyTag.symbol).length - 1;
        }
        return 0;
    }

    /// Add or remove genre
    toggleGenre(genre: string) {
        let i = this.genres.indexOf(genre);
        if (i == -1) {
            this.genres.push(genre);
        } else {
            this.genres.splice(i, 1);
        }
    }

    /// Add new custom value
    addCustom(tag: number, value: string) {
        // newly added custom value
        if (!this.custom[tag]) this.custom[tag] = [];
        if (this.custom[tag].includes(value)) return;
        // Add
        this.custom[tag].push(value);
        this.sortCustom(tag);
    } 

    /// Remove custom value
    removeCustom(tag: number, value: string) {
        // newly added custom value
        if (!this.custom[tag]) this.custom[tag] = [];
        if (!this.custom[tag].includes(value)) return;
        // Remove
        this.custom[tag].splice(this.custom[tag].indexOf(value), 1);
    }

    // Properly order the values
    sortCustom(tag: number) {
        this.custom[tag].sort((a, b) => 
            this.settings.custom[tag].values.findIndex((i) => i.val == a) - 
            this.settings.custom[tag].values.findIndex((i) => i.val == b)
        );
    }

    // Load custom tags
    loadCustom() {
        let output = [];
        for (let custom of this.settings.custom) {
            let t = this.tags[this.removeAbstractions(custom.tag.byFormat(this.format))]??[];
            // Filter atributes if multiple custom tags use the same tag
            t = t.filter(t => custom.values.findIndex(v => v.val == t) != -1);
            output.push(t);
        }
        return output;
    }

    // Get all selected custom values + note (for chips)
    getAllCustom(): CustomTagInfo[] {
        let out: CustomTagInfo[] = [];
        let i = 0;
        for (let custom of this.custom) {
            // out = out.concat(custom.filter(v => !out.includes(v) && v));
            for (let value of custom) {
                let v = value.trim()
                if (v) {
                    out.push({value: v, type: 'custom', index: i});
                }
            }
            i += 1;
        }
        // Add note tag
        // out = out.concat(this.note.split(',').filter(v => !out.includes(v) && v));
        i = 0;
        for (let value of this.processNoteString(this.note)) {
            out.push({value: value, type: 'note', index: i});
            i += 1;
        }
        return out;
    }

    // Get output tags
    getOutput() {
        let changes = [];
        // Mood change
        if (this.getMood() != this.mood) {
            changes.push({
                type: 'raw',
                tag: this.settings.moodTag.byFormat(this.format),
                value: this.mood ? [this.mood] : []
            });
        }
        // Energy change
        if (this.getEnergy() != this.energy) {
            // Rating tag
            if (this.settings.energyTag.type == 'rating') {
                changes.push({
                    type: 'rating',
                    value: this.energy??0
                });
            // Custom symbol
            } else {
                changes.push({
                    type: 'raw',
                    tag: this.settings.energyTag.tag.byFormat(this.format),
                    value: this.energy ? [this.settings.energyTag.symbol.repeat(this.energy)] : []
                });
            }
        }
        // Genre change
        if (this.isGenreChanged()) {
            // Subgenre custom tag
            if (this.settings.subgenreTag) {
                let subgenres = this.processGenreArray(
                    this.genres.filter((genre) => 
                        this.settings.genres.find((g) => g.subgenres.includes(genre))
                    )
                );
                
                let genres = this.processGenreArray(
                    this.genres.filter((g) => !subgenres.includes(g))
                );
                
                changes.push({
                    type: 'raw',
                    tag: this.settings.subgenreTag.byFormat(this.format),
                    value: subgenres
                });
                
                changes.push({
                    type: 'genre',
                    value: genres
                });
            } else {
                // Process the genres array to remove empty strings
                const cleanGenres = this.processGenreArray(this.genres);
                
                changes.push({
                    type: 'genre',
                    value: cleanGenres
                });
            }
        }
        
        // Note change
        if (this.originalNote != this.note) {
            let field = this.removeAbstractions(this.settings.noteTag.tag.byFormat(this.format));
            
            // Process original note consistently
            let original = this.processNoteString(this.originalNote);
            
            // Filter out original note values from existing tags
            let value = (this.tags[field] ?? []).filter(t => !original.includes(t));
            
            // Process new note consistently
            let noteValues = this.processNoteString(this.note);
            
            changes.push({
                type: 'raw',
                tag: field,
                value: value.concat(noteValues)
            });
        }
        
        // Custom tags
        let original = this.loadCustom();
        for (let i=0; i<original.length; i++) {
            let rawCustom = toRaw(this.custom[i]);
            if (rawCustom && ((rawCustom.length != original[i].length) || !(rawCustom.every((v, j) => original[i][j] == v)))) {
                let field = this.removeAbstractions(this.settings.custom[i].tag.byFormat(this.format));
                let values: string[] = [];
                let existingIndex = changes.findIndex(c => c.tag == field);

                // Original tag data
                if (existingIndex == -1) {
                    values = this.tags[field]??[];
                    values = values
                        .filter(v => v.trim() && !this.settings.custom[i].values.find(t => t.val == v));
                }

                // Multiple changes for the same tag
                while (existingIndex != -1) {
                    values = values.concat(changes[existingIndex].value
                        // TS throws an error because `value` can be number in case of energy, however it works
                        // @ts-ignore
                        .filter(v => !this.settings.custom[i].values.find(t => t.val == v))
                    );
                    changes.splice(existingIndex, 1);
                    existingIndex = changes.findIndex(c => c.tag == field);
                }
                changes.push({
                    type: 'raw',
                    tag: field,
                    value: values.concat(this.custom[i])
                })
            }
        }

        return {
            changes, 
            path: this.path,
            separators: this.settings.separators,
            id3v24: this.settings.id3v24,
            id3CommLang: this.settings.id3CommLang,
        };
    }

    // Whether the track has changes
    isChanged() {
        return this.getOutput().changes.length > 0
    }

}


const PLACEHOLDER_IMG = (new URL('../assets/placeholder.png', import.meta.url)).toString();

const KEY_COLORS: Record<string, string> = {
    "12A": "#00e5e5",
    "12B": "#00e5e5",
    "01A": "#00d58f",
    "01B": "#00d58f",
    "02A": "#3ffb3f",
    "02B": "#3ffb3f",
    "03A": "#97fb00",
    "03B": "#97fb00",
    "04A": "#fed600",
    "04B": "#fed600",
    "05A": "#f98c28",
    "05B": "#f98c28",
    "06A": "#fe642d",
    "06B": "#fe642d",
    "07A": "#f94949",
    "07B": "#f94949",
    "08A": "#fe3fea",
    "08B": "#fe3fea",
    "09A": "#ac64fe",
    "09B": "#ac64fe",
    "10A": "#3e89fa",
    "10B": "#3e89fa",
    "11A": "#00c9fe",
    "11B": "#00c9fe"
};

const CAMELOT_KEYS: Record<string, string> = {
    "ABM" :"01A",
    "G#M" :"01A",
    "B"   :"01B",
    "D#M" :"02A",
    "EBM" :"02A",
    "GB"  :"02B",
    "F#"  :"02B",
    "A#M" :"03A",
    "BBM" :"03A",
    "C#"  :"03B",
    "DB"  :"03B",
    "DD"  :"03B",
    "FM"  :"04A",
    "G#"  :"04B",
    "AB"  :"04B",
    "CM"  :"05A",
    "D#"  :"05B",
    "EB"  :"05B",
    "GM"  :"06A",
    "A#"  :"06B",
    "BB"  :"06B",
    "DM"  :"07A",
    "F"   :"07B",
    "AM"  :"08A",
    "C"   :"08B",
    "EM"  :"09A",
    "G"   :"09B",
    "BM"  :"10A",
    "D"   :"10B",
    "GBM" :"11A",
    "F#M" :"11A",
    "A"   :"11B",
    "C#M" :"12A",
    "DBM" :"12A",
    "E"   :"12B",
}

const OPENKEY_KEYS: Record<string, string> = {
    "5m":   "12A",
    "5d":   "12B",
    "6m":   "01A",
    "6d":   "01B",
    "7m":   "02A",
    "7d":   "02B",
    "8m":   "03A",
    "8d":   "03B",
    "9m":   "04A",
    "9d":   "04B",
    "10m":  "05A",
    "10d":  "05B",
    "11m":  "06A",
    "11d":  "06B",
    "12m":  "07A",
    "12d":  "07B",
    "1m":   "08A",
    "1d":   "08B",
    "2m":   "09A",
    "2d":   "09B",
    "3m":   "10A",
    "3d":   "10B",
    "4m":   "11A",
    "4d":   "11B"
}

// Get color for musical key
function keyColor(key?: string) {
    if (!key) return;
    key = key.trim().toUpperCase();
    // Camelot or OpenKey
    let color = KEY_COLORS[CAMELOT_KEYS[key.toUpperCase()]] || KEY_COLORS[OPENKEY_KEYS[key.toLowerCase()]];
    // Normal
    if (!color) {
        if (key.length < 3) key = `0${key}`;
        color = KEY_COLORS[key.toUpperCase()];
    }
    if (color) {
        return `color: ${color};`;
    }
}


export type { QuickTagFile, QuickTagMood, QuickTagGenre, QuickTagCustom, CustomTagInfo };
export { QuickTag, QuickTagSettings, QTTrack, EnergyTag, PLACEHOLDER_IMG, CAMELOT_KEYS, KEY_COLORS, OPENKEY_KEYS, keyColor };