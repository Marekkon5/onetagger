import Vue from 'vue';

class QTTrack {
    //From backend
    constructor(data, settings) {
        Object.assign(this, data);
        this.settings = settings;

        //Load mood, energy
        this.mood = this.getMood();
        this.energy = this.getEnergy();

        this._changes = [];
    }

    getNote() {
        return this.tags[this.settings.noteTag[this.getTagField()]];
    }

    setNote(note) {
        //Set
        let tag = this.settings.noteTag[this.getTagField()];
        this.tags[tag] = note.split(',');
        //Create change
        let index = this._changes.findIndex((c) => c.type == 'raw' && c.tag == tag);
        if (index > -1) this._changes.splice(index, 1);
        this._changes.push({
            type: 'raw',
            tag,
            value: note.split(',')
        });
    }

    //Set new genre
    setGenre(genre) {
        this.genres = [genre];
        //Generate change
        let change = {
            type: 'genre',
            value: [genre]
        }
        let index = this._changes.findIndex((c) => c.type == 'genre');
        if (index == -1) this._changes.push(change);
        else this._changes[index] = change;
    }

    //Get name of field for tag
    getTagField() {
        if (this.format == 'aiff' || this.format == 'mp3') return 'id3';
        if (this.format == 'flac') return 'vorbis';
    }

    //Get mood tag value
    getMood() {
        if (this.tags[this.settings.moodTag[this.getTagField()]]??[].length >= 1) {
            return this.tags[this.settings.moodTag[this.getTagField()]][0]
        }
        return null;
    }

    getEnergy() {
        //Use rating as energy
        if (this.settings.energyTag.type == 'rating') {
            return this.rating??0;
        }
        //Use custom symbols as energy
        if (this.tags[this.settings.energyTag[this.getTagField()]]) {
            return this.tags[this.settings.energyTag[this.getTagField()]].split(this.settings.energyTag.symbol).length - 1;
        }
        return 0;
    }

    //If has custom tag value
    hasCustom(custom, index) {
        let field = custom[this.getTagField()];
        let tag = this.tags[field];
        if (!tag) return false;
        return tag.find((t) => custom.values[index].val.toLowerCase() == t.toLowerCase());
    }

    //Toggle custom value
    toggleCustom(custom, index) {
        let field = custom[this.getTagField()];
        //Add tag
        if (!this.tags[field]) Vue.set(this.tags, field, []);
        let value = custom.values[index].val;
        let i = this.tags[field].findIndex((t) => t.toLowerCase() == value.toLowerCase());
        //Add or remove tag if exists
        if (i == -1) this.tags[field].push(value)
        else this.tags[field].splice(i, 1);
        //Clean
        this.tags[field] = this.tags[field].filter((t) => t && t.trim() != "");
        //Generate change
        let change = {
            type: 'raw',
            tag: field,
            value: this.tags[field]
        };
        //Update change
        i = this._changes.findIndex((c) => c.tag == field);
        if (i == -1) this._changes.push(change);
        else this._changes[i] = change;
    }

    //Get all selected custom values
    getAllCustom(custom) {
        let out = [];
        for(let i=0; i<custom.length; i++) {
            let field = custom[i][this.getTagField()];
            let values = this.tags[field]??[];
            //Don't add duplicate tags
            out = out.concat(values.filter((v) => !out.includes(v)));
        }
        return out;
    }

    //Get output tags
    getOutput() {
        let changes = this._changes;
        //Mood change
        if (this.getMood() != this.mood) {
            changes.push({
                type: 'raw',
                tag: this.settings.moodTag[this.getTagField()],
                value: [this.mood]
            });
        }
        //Energy change
        if (this.getEnergy() != this.energy && this.energy != 0) {
            //Rating tag
            if (this.settings.energyTag.type == 'rating') {
                changes.push({
                    type: 'rating',
                    value: this.energy
                });
            //Custom symbol
            } else {
                changes.push({
                    type: 'raw',
                    tag: this.settings.energyTag[this.getTagField()],
                    value: [this.settings.energyTag.symbol.repeat(this.energy)]
                });
            }
        }
        //Genre change
        return {changes, path: this.path};
    }

    //Wether the track has changes
    isChanged() {
        return this.getOutput().changes.length > 0
    }

    //Remove all changes (on save)
    clearChanges() {
        this._changes = [];
    }
}

export {QTTrack};