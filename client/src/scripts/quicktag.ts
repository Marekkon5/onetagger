import { toRaw } from "vue";
import { QuickTagSettings } from "./settings";
import { FrameName, Keybind, Separators } from "./utils";

class QuickTag {
    tracks: QTTrack[] = [];
    track?: QTTrack;
    failed: QuickTagFailed[] = [];
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

    // Add or remove genre
    toggleGenre(genre: string) {
        let i = this.genres.indexOf(genre);
        if (i == -1) {
            this.genres.push(genre);
        } else {
            this.genres.splice(i, 1);
        }
    }

    // Enable or disable custom value
    toggleCustom(tag: number, value: string) {
        // newly added custom value
        if (!this.custom[tag]) this.custom[tag] = [];

        let i = this.custom[tag].indexOf(value);
        // Add or remove
        if (i == -1) {
            this.custom[tag].push(value);
            this.sortCustom(tag);
        } else {
            this.custom[tag].splice(i, 1);
        }
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

    // Wether the track has changes
    isChanged() {
        return this.getOutput().changes.length > 0
    }

}


export type { QuickTagFile, QuickTagMood, QuickTagGenre, QuickTagCustom, CustomTagInfo };
export { QuickTag, QuickTagSettings, QTTrack, EnergyTag };