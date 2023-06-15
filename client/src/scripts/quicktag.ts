import { toRaw } from "vue";
import { QuickTagSettings } from "./settings";
import { FrameName, Keybind, Separators } from "./utils";

class QuickTag {
    tracks: QTTrack[] = [];
    track: QTMultiTrack = new QTMultiTrack();
    failed: QuickTagFailed[] = [];

    constructor() {}
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
            this.tracks[i].note = note;
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

        this.mood = this.getMood();
        this.energy = this.getEnergy();
        this.note = this.getNote();
        this.originalNote = this.note;
        this.custom = this.loadCustom();
        // Stupid copy
        this.originalGenres = JSON.parse(JSON.stringify(this.genres));
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
        let note = this.tags[field]??[];
        // Remove custom tags from note
        for (let custom of this.settings.custom) {
            if (custom.tag.byFormat(this.format) == field) {
                note = note.filter(v => !custom.values.map(i => i.val).includes(v));
            }
        }
        return note.join(',');
    }

    // Update note field
    setNote(note: string) {
        this.note = note;
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
        for (let value of this.note.split(',')) {
            if (value.trim())
                out.push({value, type: 'note', index: i});
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
        if (this.genres.join('') != this.originalGenres.join('')) {
            changes.push({
                type: 'genre',
                value: this.genres
            })
        }
        
        // Note change
        if (this.originalNote != this.note) {
            let field = this.removeAbstractions(this.settings.noteTag.tag.byFormat(this.format));
            // Remove original note from tags, add new one
            let original = (this.originalNote??'').split(',');
            let value = (this.tags[field]??[]).filter(t => !original.includes(t));
            changes.push({
                type: 'raw',
                tag: field,
                value: value.concat((this.note??'').split(','))
            })
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


export type { QuickTagFile, QuickTagMood, QuickTagGenre, QuickTagCustom, CustomTagInfo };
export { QuickTag, QuickTagSettings, QTTrack, EnergyTag };