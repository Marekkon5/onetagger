import { FrameName, Keybind, Separators } from "./utils";

class QuickTag {
    tracks: any[] = [];
    track?: any;
    failed: number = 0;
}

class QuickTagSettings {
    id3v24: boolean = false;
    recursive: boolean = false;
    autosave: boolean = false;
    sortDescending: boolean = false;
    sortOption: string = 'title';
    trackIndex: number = -1;
    separators: Separators = new Separators();
    noteTag: NoteTagSettings = new NoteTagSettings();

    moods: QuickTagMood[] = [
        {mood: 'Happy', color: 'amber'},
        {mood: 'Sad', color: 'indigo'},
        {mood: 'Bright', color: 'green'},
        {mood: 'Dark', color: 'deep-purple'},
        {mood: 'Angry', color: 'red'},
        {mood: 'Chill', color: 'teal'},
        {mood: 'Lovely', color: 'pink'},
        {mood: 'Powerful', color: 'light-blue'},
        {mood: 'Sexy', color: 'purple'}
    ];
    moodTag: FrameName = new FrameName('MOOD', 'TMOO', 'iTunes:MOOD');

    energyTag: EnergyTag = new EnergyTag();
    energyKeys: (Keybind | null)[] = [null, null, null, null, null];

    genres: QuickTagGenre[] = [
        {genre: '2-step', subgenres: []},
        {genre: 'Acid', subgenres: []},
        {genre: 'Breakbeat', subgenres: []},
        {genre: 'Disco', subgenres: []},
        {genre: 'Drum & Bass', subgenres: []},
        {genre: 'Electro', subgenres: ['House', 'Dubstep', 'EDM']},
        {genre: 'Funk', subgenres: []},
        {genre: 'Hardcore', subgenres: []},
        {genre: 'Hiphop', subgenres: []},
        {genre: 'House', subgenres: []},
        {genre: 'Industrial', subgenres: []},
        {genre: 'Jungle', subgenres: []},
        {genre: 'Latin', subgenres: []},
        {genre: 'Minimal', subgenres: []},
        {genre: 'Nu-Disco', subgenres: []},
        {genre: 'Oldies', subgenres: []},
        {genre: 'Pop', subgenres: []},
        {genre: 'Reggae', subgenres: []},
        {genre: 'Rock', subgenres: []},
        {genre: 'Techno', subgenres: []},
        {genre: 'Trance', subgenres: []},
    ];

    custom: QuickTagCustom[] = [
        {
            name: 'Vibe',
            tag: new FrameName('COMM', 'COMMENT', '©cmt'),
            values: [
                {val: 'Afro'},
                {val: 'Asian'},
                {val: 'Arabic'},
                {val: 'Classic'},
                {val: 'Dirty'},
                {val: 'Etnic'},
                {val: 'Funky'},
                {val: 'Gangsta'},
                {val: 'Glitchy'},
                {val: 'Melodic'},
                {val: 'Sensual'},
                {val: 'Soulful'},
            ],
        }, {
            name: 'Situation',
            tag: new FrameName('COMM', 'COMMENT', '©cmt'),
            values: [
                {val: 'Start'},
                {val: 'Build'},
                {val: 'Peak'},
                {val: 'Sustain'},
                {val: 'Release'},
            ]
        }, {
            name: 'Instruments',
            tag: new FrameName('COMM', 'COMMENT', '©cmt'),
            values: [
                {val: 'Vocals'},
                {val: 'Bass Heavy'},
                {val: 'Congas'},
                {val: 'Guitar'},
                {val: 'Horns'},
                {val: 'Organ'},
                {val: 'Piano'},
                {val: 'Strings'},
                {val: 'Sax'},
            ]
        }
    ]
}

class NoteTagSettings {
    tag: FrameName = new FrameName('COMM', 'COMMENT', '©cmt');
    keybind?: Keybind
}

interface QuickTagMood {
    mood: string;
    color: string,
    keybind?: Keybind
}

class EnergyTag {
    type: 'rating' | 'symbol' = 'rating';
    symbol: string = '*';
    tag: FrameName = new FrameName('TCOM', 'COMPOSER', '©wrt')
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

export { QuickTag, QuickTagSettings };