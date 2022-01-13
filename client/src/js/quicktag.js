class QTTrack {
    // From backend
    constructor(data, settings) {
        Object.assign(this, data);
        this.settings = settings;

        // Load mood, energy etc
        this.mood = this.getMood();
        this.energy = this.getEnergy();
        this.note = this.getNote();
        this._originalNote = this.note;
        this.custom = this.loadCustom();
        // very retarded, but it is what it is
        this._genres = JSON.parse(JSON.stringify(this.genres));
    }

    // Get note from tags
    getNote() {
        if (this.note || this.note === '') {
            return this.note;
        }
        let field = this.removeAbstractions(this.settings.noteTag.tag[this.getTagField()]);
        let note = this.tags[field]??[];
        // Remove custom tags from note
        for (let custom of this.settings.custom) {
            if (custom.tag[this.getTagField()] == field) {
                note = note.filter(v => !custom.values.map(i => i.val).includes(v));
            }
        }
        return note.join(',');
    }

    // Update note field
    setNote(note) {
        this.note = note;
    }

    // Get name of field for tag
    getTagField() {
        switch (this.format) {
            case 'mp3':
            case 'aiff':
                return 'id3';
            case 'flac':
                return 'vorbis';
            case 'mp4':
                return 'mp4';
        }
    }

    removeAbstractions(input) {
        if (this.format != 'mp4' || !input) return input;
        // Leading
        input = input.replace('----:', '');
        // iTunes:VALUE -> com.apple.Itunes:VALUE
        if (input.startsWith('iTunes:')) input = 'com.apple.' + input;
        return input;
    }

    // Get mood tag value
    getMood() {
        let field = this.removeAbstractions(this.settings.moodTag[this.getTagField()]);
        if (this.tags[field]??[].length >= 1) {
            return this.tags[field][0]
        }
        return null;
    }

    getEnergy() {
        // Use rating as energy
        if (this.settings.energyTag.type == 'rating') {
            return this.rating??0;
        }
        // Use custom symbols as energy
        let t = this.tags[this.removeAbstractions(this.settings.energyTag.tag[this.getTagField()])];
        if (t) {
            // Use first element of array
            if (typeof t == 'object') {
                if (t.length == 0) return 0;
                t = t[0];
            }
            return t.split(this.settings.energyTag.symbol).length - 1;
        }
        return 0;
    }

    // Enable or disable custom value
    toggleCustom(tag, value) {
        let i = this.custom[tag].indexOf(value);
        // Add or remove
        if (i == -1) {
            this.custom[tag].push(value);
            this.sortCustom(tag);
        } else {
            this.custom[tag].splice(i, 1);
        }
    }

    // Add or remove genre
    toggleGenre(genre) {
        let i = this.genres.indexOf(genre);
        if (i == -1) {
            this.genres.push(genre);
        } else {
            this.genres.splice(i, 1);
        }
    }

    // Properly order the values
    sortCustom(tag) {
        this.custom[tag].sort((a, b) => 
            this.settings.custom[tag].values.findIndex((i) => i.val == a) - 
            this.settings.custom[tag].values.findIndex((i) => i.val == b)
        );
    }

    // Load custom tags
    loadCustom() {
        let output = [];
        for (let custom of this.settings.custom) {
            let t = this.tags[this.removeAbstractions(custom.tag[this.getTagField()])]??[];
            // Filter atributes if multiple custom tags use the same tag
            t = t.filter(t => custom.values.findIndex(v => v.val == t) != -1)
            output.push(t);
        }
        return output;
    }

    // Get all selected custom values + note (for chips)
    getAllCustom() {
        let out = [];
        for (let custom of this.custom) {
            out = out.concat(custom.filter(v => !out.includes(v) && v));
        }
        // Add note tag
        out = out.concat(this.note.split(',').filter(v => !out.includes(v) && v));
        return out;
    }

    // Get output tags
    getOutput() {
        let changes = [];
        // Mood change
        if (this.getMood() != this.mood) {
            changes.push({
                type: 'raw',
                tag: this.settings.moodTag[this.getTagField()],
                value: [this.mood]
            });
        }
        // Energy change
        if (this.getEnergy() != this.energy && this.energy != 0) {
            // Rating tag
            if (this.settings.energyTag.type == 'rating') {
                changes.push({
                    type: 'rating',
                    value: this.energy
                });
            // Custom symbol
            } else {
                changes.push({
                    type: 'raw',
                    tag: this.settings.energyTag.tag[this.getTagField()],
                    value: [this.settings.energyTag.symbol.repeat(this.energy)]
                });
            }
        }
        // Genre change
        if (this.genres.join('') != this._genres.join('')) {
            changes.push({
                type: 'genre',
                value: this.genres
            })
        }
        
        // Note change
        if (this._originalNote != this.note) {
            let field = this.removeAbstractions(this.settings.noteTag.tag[this.getTagField()]);
            // Remove original note from tags, add new one
            let original = this._originalNote.split(',');
            let value = (this.tags[field]??[]).filter(t => !original.includes(t));
            changes.push({
                type: 'raw',
                tag: field,
                value: value.concat(this.note.split(','))
            })
        }
        
        // Custom tags
        let original = this.loadCustom();
        for(let i=0; i<original.length; i++) {
            if (original[i].length != this.custom[i].length) {
                
                let field = this.removeAbstractions(this.settings.custom[i].tag[this.getTagField()]);
                let values = [];
                let existingIndex = changes.findIndex(c => c.tag == field);
                // Original tag data
                if (existingIndex == -1) {
                    values = this.tags[field]??[];
                    values = values.filter(v => !this.settings.custom[i].values.find(t => t.val == v));
                }
                // Multiple changes for the same tag
                while (existingIndex != -1) {
                    values = values.concat(changes[existingIndex].value
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
            separators: this.settings.separators
        };
    }

    // Wether the track has changes
    isChanged() {
        return this.getOutput().changes.length > 0
    }

}

export {QTTrack};