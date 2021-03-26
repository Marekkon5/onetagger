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
}

export {QTTrack};