// Cryptographic utilities for IPPAN address generation
// Browser-compatible implementation using Web Crypto API

// IPPAN address format: starts with 'i' followed by 64 hex characters
const IPPAN_PREFIX = 'i'
const ADDRESS_LENGTH = 65 // Total length: 1 + 64 hex chars

// Simple base58 encoding (Bitcoin style)
const BASE58_ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'

function base58Encode(buffer: Uint8Array): string {
  if (buffer.length === 0) return ''
  
  let num = BigInt('0x' + Array.from(buffer).map(b => b.toString(16).padStart(2, '0')).join(''))
  let encoded = ''
  
  while (num > 0n) {
    const remainder = num % 58n
    num = num / 58n
    encoded = BASE58_ALPHABET[Number(remainder)] + encoded
  }
  
  // Add leading '1's for leading zeros
  for (let i = 0; i < buffer.length && buffer[i] === 0; i++) {
    encoded = '1' + encoded
  }
  
  return encoded
}

function base58Decode(encoded: string): Uint8Array {
  let num = 0n
  let multi = 1n
  
  for (let i = encoded.length - 1; i >= 0; i--) {
    const char = encoded[i]
    const index = BASE58_ALPHABET.indexOf(char)
    if (index === -1) throw new Error('Invalid base58 character')
    num += BigInt(index) * multi
    multi *= 58n
  }
  
  const hex = num.toString(16)
  const buffer = new Uint8Array(hex.length / 2)
  for (let i = 0; i < hex.length; i += 2) {
    buffer[i / 2] = parseInt(hex.substr(i, 2), 16)
  }
  
  // Add leading zeros
  const leadingOnes = encoded.match(/^1+/)?.[0].length || 0
  const result = new Uint8Array(leadingOnes + buffer.length)
  result.set(buffer, leadingOnes)
  return result
}

// Generate a random private key (32 bytes)
export function generatePrivateKey(): string {
  const array = new Uint8Array(32)
  crypto.getRandomValues(array)
  return Array.from(array).map(b => b.toString(16).padStart(2, '0')).join('')
}

// Hash function using Web Crypto API
async function sha256(data: Uint8Array): Promise<Uint8Array> {
  const hashBuffer = await crypto.subtle.digest('SHA-256', data)
  return new Uint8Array(hashBuffer)
}

// Derive public key from private key using simple ECDSA-like approach
export async function derivePublicKey(privateKey: string): Promise<string> {
  const privateKeyBuffer = new Uint8Array(32)
  for (let i = 0; i < 32; i++) {
    privateKeyBuffer[i] = parseInt(privateKey.substr(i * 2, 2), 16)
  }
  
  const hash = await sha256(privateKeyBuffer)
  
  // Simple key derivation (in real implementation, use proper ECDSA)
  const combined = new Uint8Array(hash.length + 16) // 16 bytes for 'IPPAN_PUBLIC_KEY'
  combined.set(hash)
  const text = new TextEncoder().encode('IPPAN_PUBLIC_KEY')
  combined.set(text, hash.length)
  
  const publicKey = await sha256(combined)
  return Array.from(publicKey).map(b => b.toString(16).padStart(2, '0')).join('')
}

// Generate IPPAN address from public key
export async function generateAddress(publicKey: string): Promise<string> {
  const publicKeyBuffer = new Uint8Array(32)
  for (let i = 0; i < 32; i++) {
    publicKeyBuffer[i] = parseInt(publicKey.substr(i * 2, 2), 16)
  }
  
  // Create address hash
  const combined = new Uint8Array(publicKeyBuffer.length + 13) // 13 bytes for 'IPPAN_ADDRESS'
  combined.set(publicKeyBuffer)
  const text = new TextEncoder().encode('IPPAN_ADDRESS')
  combined.set(text, publicKeyBuffer.length)
  
  const addressHash = await sha256(combined)
  
  // Take first 32 bytes for address (IPPAN uses 32-byte addresses)
  const addressBytes = addressHash.slice(0, 32)
  
  // Convert to hex string
  const hexAddress = Array.from(addressBytes).map(b => b.toString(16).padStart(2, '0')).join('')
  
  // Add IPPAN prefix
  return `${IPPAN_PREFIX}${hexAddress}`
}

// Generate a complete wallet (private key, public key, address)
export async function generateWallet(): Promise<{
  privateKey: string
  publicKey: string
  address: string
  seedPhrase: string
}> {
  const privateKey = generatePrivateKey()
  const publicKey = await derivePublicKey(privateKey)
  const address = await generateAddress(publicKey)
  const seedPhrase = generateSeedPhrase()
  
  return {
    privateKey,
    publicKey,
    address,
    seedPhrase
  }
}

// Generate a 12-word seed phrase
export function generateSeedPhrase(): string {
  const wordList = [
    'abandon', 'ability', 'able', 'about', 'above', 'absent', 'absorb', 'abstract', 'absurd', 'abuse',
    'access', 'accident', 'account', 'accuse', 'achieve', 'acid', 'acoustic', 'acquire', 'across', 'act',
    'action', 'actor', 'actress', 'actual', 'adapt', 'add', 'addict', 'address', 'adjust', 'admit',
    'adult', 'advance', 'advice', 'aerobic', 'affair', 'afford', 'afraid', 'again', 'age', 'agent',
    'agree', 'ahead', 'aim', 'air', 'airport', 'aisle', 'alarm', 'album', 'alcohol', 'alert',
    'alien', 'all', 'alley', 'allow', 'almost', 'alone', 'alpha', 'already', 'also', 'alter',
    'always', 'amateur', 'amazing', 'among', 'amount', 'amused', 'analyst', 'anchor', 'ancient', 'anger',
    'angle', 'angry', 'animal', 'ankle', 'announce', 'annual', 'another', 'answer', 'antenna', 'antique',
    'anxiety', 'any', 'apart', 'apology', 'appear', 'apple', 'approve', 'april', 'arch', 'arctic',
    'area', 'arena', 'argue', 'arm', 'armed', 'armor', 'army', 'around', 'arrange', 'arrest',
    'arrive', 'arrow', 'art', 'artefact', 'artist', 'artwork', 'ask', 'aspect', 'assault', 'asset',
    'assist', 'assume', 'asthma', 'athlete', 'atom', 'attack', 'attend', 'attitude', 'attract', 'auction',
    'audit', 'august', 'aunt', 'author', 'auto', 'autumn', 'average', 'avocado', 'avoid', 'awake',
    'aware', 'away', 'awesome', 'awful', 'awkward', 'axis', 'baby', 'bachelor', 'bacon', 'badge',
    'bag', 'balance', 'balcony', 'ball', 'bamboo', 'banana', 'banner', 'bar', 'barely', 'bargain',
    'barrel', 'base', 'basic', 'basket', 'battle', 'beach', 'bean', 'beauty', 'because', 'become',
    'beef', 'before', 'begin', 'behave', 'behind', 'believe', 'below', 'belt', 'bench', 'benefit',
    'best', 'betray', 'better', 'between', 'beyond', 'bicycle', 'bid', 'bike', 'bind', 'biology',
    'bird', 'birth', 'bitter', 'black', 'blade', 'blame', 'blanket', 'blast', 'bleak', 'bless',
    'blind', 'blood', 'blossom', 'blow', 'blue', 'blur', 'blush', 'board', 'boat', 'body',
    'boil', 'bomb', 'bone', 'bonus', 'book', 'boost', 'border', 'boring', 'borrow', 'boss',
    'bottom', 'bounce', 'box', 'boy', 'bracket', 'brain', 'brand', 'brass', 'brave', 'bread',
    'breeze', 'brick', 'bridge', 'brief', 'bright', 'bring', 'brisk', 'broccoli', 'broken', 'bronze',
    'broom', 'brother', 'brown', 'brush', 'bubble', 'buddy', 'budget', 'buffalo', 'build', 'bulb',
    'bulk', 'bullet', 'bundle', 'bunker', 'burden', 'burger', 'burst', 'bus', 'business', 'busy',
    'butter', 'buyer', 'buzz', 'cabbage', 'cabin', 'cable', 'cactus', 'cage', 'cake', 'call',
    'calm', 'camera', 'camp', 'can', 'canal', 'cancel', 'candy', 'cannon', 'canoe', 'canvas',
    'canyon', 'capable', 'capital', 'captain', 'car', 'carbon', 'card', 'cargo', 'carpet', 'carry',
    'cart', 'case', 'cash', 'casino', 'cast', 'casual', 'cat', 'catalog', 'catch', 'category',
    'cattle', 'caught', 'cause', 'caution', 'cave', 'ceiling', 'celery', 'cement', 'census', 'century',
    'cereal', 'certain', 'chair', 'chalk', 'champion', 'change', 'chaos', 'chapter', 'charge', 'chase',
    'cheap', 'check', 'cheese', 'chef', 'cherry', 'chest', 'chicken', 'chief', 'child', 'chimney',
    'choice', 'choose', 'chronic', 'chuckle', 'chunk', 'churn', 'cigar', 'cinnamon', 'circle', 'citizen',
    'city', 'civil', 'claim', 'clamp', 'clarify', 'claw', 'clay', 'clean', 'clerk', 'clever',
    'click', 'client', 'cliff', 'climb', 'clinic', 'clip', 'clock', 'clog', 'close', 'cloth',
    'cloud', 'clown', 'club', 'clump', 'cluster', 'clutch', 'coach', 'coast', 'coconut', 'code',
    'coffee', 'coil', 'coin', 'collect', 'color', 'column', 'come', 'comfort', 'comic', 'common',
    'company', 'concert', 'conduct', 'confirm', 'congress', 'connect', 'consider', 'control', 'convince', 'cook',
    'cool', 'copper', 'copy', 'coral', 'core', 'corn', 'correct', 'cost', 'cotton', 'couch',
    'country', 'couple', 'course', 'cousin', 'cover', 'coyote', 'crack', 'cradle', 'craft', 'cram',
    'crane', 'crash', 'crater', 'crawl', 'crazy', 'cream', 'credit', 'creek', 'crew', 'cricket',
    'crime', 'crisp', 'critic', 'crop', 'cross', 'crouch', 'crowd', 'crucial', 'cruel', 'cruise',
    'crumble', 'crunch', 'crush', 'cry', 'crystal', 'cube', 'culture', 'cup', 'cupboard', 'curious',
    'current', 'curtain', 'curve', 'cushion', 'custom', 'cute', 'cycle', 'dad', 'damage', 'damp',
    'dance', 'danger', 'daring', 'dash', 'daughter', 'dawn', 'day', 'deal', 'debate', 'debris',
    'decade', 'december', 'decide', 'decline', 'decorate', 'decrease', 'deer', 'defense', 'define', 'defy',
    'degree', 'delay', 'deliver', 'demand', 'demise', 'denial', 'dentist', 'deny', 'depart', 'depend',
    'deposit', 'depth', 'deputy', 'derive', 'describe', 'desert', 'design', 'desk', 'despair', 'destroy',
    'detail', 'detect', 'develop', 'device', 'devote', 'diagram', 'dial', 'diamond', 'diary', 'dice',
    'diesel', 'diet', 'differ', 'digital', 'dignity', 'dilemma', 'dinner', 'dinosaur', 'direct', 'dirt',
    'disagree', 'discover', 'disease', 'dish', 'dismiss', 'disorder', 'display', 'distance', 'divert', 'divide',
    'divorce', 'dizzy', 'doctor', 'document', 'dog', 'doll', 'dolphin', 'domain', 'donate', 'donkey',
    'donor', 'door', 'dose', 'double', 'dove', 'draft', 'dragon', 'drama', 'drastic', 'draw',
    'dream', 'dress', 'drift', 'drill', 'drink', 'drip', 'drive', 'drop', 'drum', 'dry',
    'duck', 'dumb', 'dune', 'during', 'dust', 'dutch', 'duty', 'dwarf', 'dynamic', 'eager',
    'eagle', 'early', 'earn', 'earth', 'easily', 'east', 'easy', 'echo', 'ecology', 'economy',
    'edge', 'edit', 'educate', 'effort', 'egg', 'eight', 'either', 'elbow', 'elder', 'electric',
    'elegant', 'element', 'elephant', 'elevator', 'elite', 'else', 'embark', 'embody', 'embrace', 'emerge',
    'emotion', 'employ', 'empower', 'empty', 'enable', 'enact', 'end', 'endless', 'endorse', 'enemy',
    'energy', 'enforce', 'engage', 'engine', 'english', 'enjoy', 'enlist', 'enough', 'enrich', 'enroll',
    'ensure', 'enter', 'entire', 'entry', 'envelope', 'episode', 'equal', 'equip', 'era', 'erase',
    'erode', 'erosion', 'erupt', 'escape', 'essay', 'essence', 'estate', 'eternal', 'ethics', 'evidence',
    'evil', 'evoke', 'evolve', 'exact', 'example', 'excess', 'exchange', 'excite', 'exclude', 'excuse',
    'execute', 'exercise', 'exhaust', 'exhibit', 'exile', 'exist', 'exit', 'exotic', 'expand', 'expect',
    'expire', 'explain', 'expose', 'express', 'extend', 'extra', 'eye', 'eyebrow', 'fabric', 'face',
    'faculty', 'fade', 'faint', 'faith', 'fall', 'false', 'fame', 'family', 'famous', 'fan',
    'fancy', 'fantasy', 'farm', 'fashion', 'fat', 'fatal', 'father', 'fatigue', 'fault', 'favorite',
    'feature', 'february', 'federal', 'fee', 'feed', 'feel', 'female', 'fence', 'festival', 'fetch',
    'fever', 'few', 'fiber', 'fiction', 'field', 'figure', 'file', 'film', 'filter', 'final',
    'find', 'fine', 'finger', 'finish', 'fire', 'firm', 'first', 'fiscal', 'fish', 'fishing',
    'fit', 'fitness', 'fix', 'flag', 'flame', 'flash', 'flat', 'flavor', 'flee', 'flight',
    'flip', 'float', 'flock', 'floor', 'flower', 'fluid', 'flush', 'fly', 'foam', 'focus',
    'fog', 'foil', 'fold', 'follow', 'food', 'foot', 'force', 'forest', 'forget', 'fork',
    'fortune', 'forum', 'forward', 'fossil', 'foster', 'found', 'fox', 'fragile', 'frame', 'frequent',
    'fresh', 'friend', 'fringe', 'frog', 'front', 'frost', 'frown', 'frozen', 'fruit', 'fuel',
    'fun', 'funny', 'furnace', 'fury', 'future', 'gadget', 'gain', 'galaxy', 'gallery', 'game',
    'gap', 'garage', 'garbage', 'garden', 'garlic', 'garment', 'gas', 'gasp', 'gate', 'gather',
    'gauge', 'gaze', 'general', 'genius', 'genre', 'gentle', 'genuine', 'gesture', 'ghost', 'giant',
    'gift', 'giggle', 'ginger', 'giraffe', 'girl', 'give', 'glad', 'glance', 'glare', 'glass',
    'glide', 'glimpse', 'globe', 'gloom', 'glory', 'glove', 'glow', 'glue', 'goat', 'goddess',
    'gold', 'good', 'goose', 'gorilla', 'gospel', 'gossip', 'govern', 'gown', 'grab', 'grace',
    'grain', 'grant', 'grape', 'grass', 'gravity', 'great', 'green', 'grid', 'grief', 'grit',
    'grocery', 'group', 'grow', 'grunt', 'guard', 'guess', 'guide', 'guilt', 'guitar', 'gun',
    'gym', 'habit', 'hair', 'half', 'hammer', 'hamster', 'hand', 'happy', 'harbor', 'hard',
    'harsh', 'harvest', 'hash', 'hat', 'hate', 'have', 'hawk', 'head', 'health', 'heart',
    'heavy', 'hedgehog', 'height', 'hello', 'helmet', 'help', 'hen', 'hero', 'hidden', 'high',
    'hill', 'hint', 'hip', 'hire', 'history', 'hobby', 'hockey', 'hold', 'hole', 'holiday',
    'hollow', 'home', 'honey', 'hood', 'hope', 'horn', 'horror', 'horse', 'hospital', 'host',
    'hotel', 'hour', 'hover', 'hub', 'huge', 'human', 'humble', 'humor', 'hundred', 'hungry',
    'hunt', 'hurdle', 'hurry', 'hurt', 'husband', 'hybrid', 'ice', 'icon', 'idea', 'identify',
    'idle', 'ignore', 'ill', 'illegal', 'illness', 'image', 'imitate', 'immense', 'immune', 'impact',
    'impose', 'improve', 'impulse', 'inch', 'include', 'income', 'increase', 'index', 'indicate', 'indoor',
    'industry', 'infant', 'inflict', 'inform', 'inhale', 'inherit', 'initial', 'inject', 'injury', 'inmate',
    'inner', 'innocent', 'input', 'inquiry', 'insane', 'insect', 'inside', 'inspire', 'install', 'intact',
    'interest', 'into', 'invest', 'invite', 'involve', 'iron', 'island', 'isolate', 'issue', 'item',
    'ivory', 'jacket', 'jaguar', 'jar', 'jazz', 'jealous', 'jeans', 'jelly', 'jewel', 'job',
    'join', 'joke', 'journey', 'joy', 'judge', 'juice', 'jump', 'jungle', 'junior', 'junk',
    'just', 'kangaroo', 'keen', 'keep', 'ketchup', 'key', 'kick', 'kid', 'kidney', 'kind',
    'kingdom', 'kiss', 'kit', 'kitchen', 'kite', 'kitten', 'kiwi', 'knee', 'knife', 'knock',
    'know', 'lab', 'label', 'labor', 'ladder', 'lady', 'lake', 'lamp', 'land', 'large',
    'laser', 'late', 'latin', 'laugh', 'laundry', 'lava', 'law', 'lawn', 'lawsuit', 'layer',
    'lazy', 'leader', 'leaf', 'learn', 'leave', 'lecture', 'left', 'leg', 'legal', 'legend',
    'leisure', 'lemon', 'lend', 'length', 'lens', 'leopard', 'lesson', 'letter', 'level', 'liar',
    'liberty', 'library', 'license', 'life', 'lift', 'light', 'like', 'limb', 'limit', 'link',
    'lion', 'liquid', 'list', 'little', 'live', 'lizard', 'load', 'loan', 'lobster', 'local',
    'lock', 'logic', 'lonely', 'long', 'loop', 'lottery', 'loud', 'lounge', 'love', 'loyal',
    'lucky', 'luggage', 'lumber', 'lunar', 'lunch', 'luxury', 'lyrics', 'machine', 'mad', 'magic',
    'magnet', 'maid', 'mail', 'main', 'major', 'make', 'mammal', 'man', 'manage', 'mandate',
    'mango', 'mansion', 'manual', 'maple', 'marble', 'march', 'margin', 'marine', 'market', 'marriage',
    'mask', 'mass', 'master', 'match', 'material', 'math', 'matrix', 'matter', 'maximum', 'maze',
    'meadow', 'mean', 'measure', 'meat', 'mechanic', 'medal', 'media', 'melody', 'melt', 'member',
    'memory', 'mention', 'menu', 'mercy', 'merge', 'merit', 'merry', 'mesh', 'message', 'metal',
    'method', 'middle', 'midnight', 'milk', 'million', 'mimic', 'mind', 'minimum', 'minor', 'minute',
    'miracle', 'mirror', 'misery', 'miss', 'mistake', 'mix', 'mixed', 'mixture', 'mobile', 'model',
    'modify', 'mom', 'moment', 'monitor', 'monkey', 'monster', 'month', 'moon', 'moral', 'more',
    'morning', 'mosquito', 'mother', 'motion', 'motor', 'mountain', 'mouse', 'move', 'movie', 'much',
    'muffin', 'mule', 'multiply', 'muscle', 'museum', 'mushroom', 'music', 'must', 'mutual', 'myself',
    'mystery', 'naive', 'name', 'napkin', 'narrow', 'nasty', 'nation', 'nature', 'near', 'neck',
    'need', 'negative', 'neglect', 'neither', 'nephew', 'nerve', 'nest', 'net', 'network', 'neutral',
    'never', 'news', 'next', 'nice', 'night', 'noble', 'noise', 'nominee', 'noodle', 'normal',
    'north', 'nose', 'notable', 'note', 'nothing', 'notice', 'novel', 'now', 'nuclear', 'number',
    'nurse', 'nut', 'oak', 'obey', 'object', 'oblige', 'obscure', 'observe', 'obtain', 'obvious',
    'occur', 'ocean', 'october', 'odor', 'off', 'offer', 'office', 'often', 'oil', 'okay',
    'old', 'olive', 'olympic', 'omit', 'once', 'one', 'onion', 'online', 'only', 'open',
    'opera', 'opinion', 'opponent', 'opportunity', 'oppose', 'option', 'orange', 'orbit', 'orchard', 'order',
    'ordinary', 'organ', 'orient', 'original', 'orphan', 'ostrich', 'other', 'our', 'ours', 'ourselves',
    'out', 'outdoor', 'outer', 'outfit', 'outing', 'outline', 'outlook', 'output', 'outrage', 'outset',
    'outside', 'outward', 'oval', 'oven', 'over', 'own', 'owner', 'ox', 'oxygen', 'oyster',
    'ozone', 'pact', 'paddle', 'page', 'pair', 'palace', 'palm', 'panda', 'panel', 'panic',
    'panther', 'paper', 'parade', 'parent', 'park', 'parrot', 'party', 'pass', 'patch', 'path',
    'patient', 'patrol', 'pattern', 'pause', 'pave', 'payment', 'peace', 'peanut', 'pear', 'peasant',
    'pelican', 'pen', 'penalty', 'pencil', 'people', 'pepper', 'perfect', 'perform', 'perhaps', 'period',
    'permit', 'person', 'pet', 'phone', 'photo', 'phrase', 'physical', 'piano', 'picnic', 'picture',
    'piece', 'pig', 'pigeon', 'pill', 'pilot', 'pink', 'pioneer', 'pipe', 'pistol', 'pitch',
    'pizza', 'place', 'planet', 'plastic', 'plate', 'play', 'please', 'pledge', 'pluck', 'plug',
    'plunge', 'poem', 'poet', 'point', 'polar', 'pole', 'police', 'pond', 'pony', 'pool',
    'poor', 'pop', 'pope', 'popular', 'portion', 'position', 'possible', 'post', 'potato', 'pottery',
    'poverty', 'powder', 'power', 'practice', 'praise', 'predict', 'prefer', 'prepare', 'present', 'pretty',
    'prevent', 'price', 'pride', 'primary', 'print', 'priority', 'prison', 'private', 'prize', 'problem',
    'process', 'produce', 'profit', 'program', 'project', 'promote', 'proof', 'property', 'prosper', 'protect',
    'proud', 'provide', 'public', 'pudding', 'pull', 'pulp', 'pulse', 'pumpkin', 'punch', 'pupil',
    'puppy', 'purchase', 'purple', 'purpose', 'purse', 'push', 'put', 'puzzle', 'pyramid', 'quality',
    'quantum', 'quarter', 'question', 'quick', 'quit', 'quiz', 'quote', 'rabbit', 'raccoon', 'race',
    'rack', 'radar', 'radio', 'rail', 'rain', 'raise', 'rally', 'ramp', 'ranch', 'random',
    'range', 'rapid', 'rare', 'rate', 'rather', 'raven', 'raw', 'razor', 'ready', 'real',
    'reason', 'rebel', 'rebuild', 'recall', 'receive', 'recipe', 'record', 'recover', 'recycle', 'red',
    'reduce', 'reflect', 'reform', 'refuse', 'region', 'regret', 'regular', 'reject', 'relax', 'release',
    'relief', 'rely', 'remain', 'remember', 'remind', 'remove', 'render', 'renew', 'rent', 'reopen',
    'repair', 'repeat', 'replace', 'reply', 'report', 'require', 'rescue', 'resemble', 'resist', 'resource',
    'response', 'result', 'retire', 'retreat', 'return', 'reunion', 'reveal', 'review', 'reward', 'rhythm',
    'rib', 'ribbon', 'rice', 'rich', 'ride', 'ridge', 'rifle', 'right', 'rigid', 'ring',
    'riot', 'ripple', 'risk', 'ritual', 'rival', 'river', 'road', 'roast', 'robot', 'robust',
    'rocket', 'romance', 'roof', 'rookie', 'room', 'rose', 'rotate', 'rough', 'round', 'route',
    'row', 'royal', 'rubber', 'rude', 'rug', 'rule', 'run', 'runway', 'rural', 'sad',
    'saddle', 'sadness', 'safe', 'sail', 'salad', 'salmon', 'salon', 'salt', 'salute', 'same',
    'sample', 'sand', 'satisfy', 'satoshi', 'sauce', 'sausage', 'save', 'say', 'scale', 'scan',
    'scare', 'scatter', 'scene', 'scheme', 'school', 'science', 'scissors', 'scorpion', 'scout', 'scrap',
    'screen', 'script', 'scrub', 'sea', 'search', 'season', 'seat', 'second', 'secret', 'section',
    'security', 'seed', 'seek', 'segment', 'select', 'sell', 'seminar', 'senior', 'sense', 'sentence',
    'series', 'service', 'session', 'settle', 'setup', 'seven', 'shadow', 'shaft', 'shallow', 'share',
    'shed', 'shell', 'sheriff', 'shield', 'shift', 'shine', 'ship', 'shiver', 'shock', 'shoe',
    'shoot', 'shop', 'shore', 'short', 'shoulder', 'shove', 'shrimp', 'shrug', 'shuffle', 'shy',
    'sibling', 'sick', 'side', 'siege', 'sight', 'sign', 'silent', 'silk', 'silly', 'silver',
    'similar', 'simple', 'since', 'sing', 'siren', 'sister', 'situate', 'six', 'size', 'skate',
    'sketch', 'ski', 'skill', 'skin', 'skirt', 'skull', 'slab', 'slam', 'sleep', 'slender',
    'slice', 'slide', 'slight', 'slim', 'slogan', 'slot', 'slow', 'slurp', 'slush', 'small',
    'smart', 'smile', 'smoke', 'smooth', 'smuggle', 'snack', 'snake', 'snap', 'snare', 'snarl',
    'sneak', 'sneeze', 'sniff', 'snore', 'snort', 'snow', 'snug', 'snuggle', 'soak', 'soap',
    'soar', 'sob', 'soccer', 'social', 'sock', 'soda', 'soft', 'soggy', 'soil', 'solar',
    'soldier', 'solid', 'solo', 'solve', 'someone', 'song', 'soon', 'sorry', 'sort', 'soul',
    'sound', 'soup', 'sour', 'south', 'space', 'spare', 'spark', 'spatial', 'spawn', 'speak',
    'speed', 'spell', 'spend', 'sphere', 'spice', 'spider', 'spike', 'spin', 'spirit', 'spit',
    'splash', 'spoil', 'sponsor', 'spoon', 'sport', 'spot', 'spray', 'spread', 'spring', 'spy',
    'square', 'squeeze', 'squirrel', 'squirt', 'stable', 'stadium', 'staff', 'stage', 'stain', 'stair',
    'stake', 'stale', 'stall', 'stamp', 'stand', 'start', 'state', 'stay', 'steak', 'steel',
    'steep', 'steer', 'step', 'stereo', 'stick', 'still', 'sting', 'stink', 'stir', 'stitch',
    'stock', 'stomach', 'stone', 'stool', 'stoop', 'stop', 'store', 'storm', 'story', 'stove',
    'straddle', 'straight', 'strain', 'strand', 'strange', 'strategy', 'straw', 'stream', 'street', 'strength',
    'stress', 'stretch', 'strict', 'stride', 'strike', 'string', 'strive', 'stroke', 'stroll', 'strong',
    'struggle', 'strut', 'stuck', 'student', 'stuff', 'stumble', 'stun', 'stunt', 'style', 'subject',
    'submit', 'subway', 'success', 'such', 'sudden', 'suffer', 'sugar', 'suggest', 'suit', 'sultan',
    'sum', 'sun', 'sunny', 'sunset', 'super', 'supply', 'supreme', 'sure', 'surface', 'surge',
    'surprise', 'surround', 'survey', 'suspect', 'sustain', 'swallow', 'swamp', 'swap', 'swarm', 'sway',
    'swear', 'sweat', 'sweep', 'sweet', 'swell', 'swift', 'swim', 'swing', 'switch', 'sword',
    'swore', 'sworn', 'swung', 'syllable', 'symbol', 'symptom', 'syrup', 'system', 'table', 'tackle',
    'tag', 'tail', 'talent', 'talk', 'tall', 'tank', 'tape', 'target', 'task', 'taste',
    'tattoo', 'taxi', 'teach', 'team', 'tell', 'ten', 'tenant', 'tennis', 'tent', 'term',
    'test', 'text', 'thank', 'that', 'the', 'their', 'them', 'then', 'theory', 'there',
    'they', 'thing', 'think', 'third', 'this', 'thorough', 'those', 'though', 'thought', 'thousand',
    'thread', 'threat', 'three', 'thrive', 'throw', 'thumb', 'thunder', 'ticket', 'tide', 'tidy',
    'tie', 'tiger', 'tight', 'tile', 'till', 'timber', 'time', 'tiny', 'tip', 'tire',
    'tired', 'tissue', 'title', 'toast', 'tobacco', 'today', 'toddler', 'toe', 'together', 'toilet',
    'token', 'tomato', 'tomorrow', 'tone', 'tongue', 'tonight', 'tool', 'tooth', 'top', 'topic',
    'topple', 'torch', 'tornado', 'tortoise', 'toss', 'total', 'touch', 'tough', 'tour', 'toward',
    'town', 'toy', 'track', 'trade', 'traffic', 'tragic', 'train', 'transfer', 'trap', 'trash',
    'travel', 'tray', 'treat', 'tree', 'trend', 'trial', 'tribe', 'trick', 'trigger', 'trim',
    'trip', 'trophy', 'trouble', 'truck', 'true', 'truly', 'trumpet', 'trust', 'truth', 'try',
    'tube', 'tuesday', 'tug', 'tuition', 'tulip', 'tumble', 'tuna', 'tunnel', 'turbo', 'turtle',
    'twelve', 'twenty', 'twice', 'twin', 'twist', 'two', 'type', 'typical', 'ugly', 'umbrella',
    'unable', 'unaware', 'uncle', 'uncover', 'under', 'undo', 'unfair', 'unfold', 'unhappy', 'unique',
    'unit', 'universe', 'unknown', 'unlock', 'until', 'unusual', 'unveil', 'update', 'upgrade', 'uphold',
    'upon', 'upper', 'upright', 'upset', 'urban', 'urge', 'usage', 'use', 'used', 'useful',
    'useless', 'usual', 'utility', 'vacant', 'vacuum', 'vague', 'vain', 'valid', 'valley', 'valve',
    'van', 'vanish', 'vapor', 'various', 'vast', 'vault', 'vehicle', 'velvet', 'vendor', 'venom',
    'venture', 'venue', 'verb', 'verify', 'version', 'very', 'vessel', 'veteran', 'viable', 'vibrant',
    'vicious', 'victory', 'video', 'view', 'village', 'vintage', 'violin', 'virtual', 'virus', 'visa',
    'visit', 'visual', 'vital', 'vivid', 'vocal', 'voice', 'void', 'volcano', 'volume', 'vote',
    'voyage', 'wage', 'wagon', 'wait', 'wake', 'walk', 'wall', 'walnut', 'want', 'war',
    'warm', 'warrior', 'wash', 'wasp', 'waste', 'water', 'wave', 'way', 'weak', 'wealth',
    'weapon', 'wear', 'weasel', 'weather', 'web', 'wedding', 'weed', 'week', 'weird', 'welcome',
    'west', 'wet', 'whale', 'what', 'wheat', 'wheel', 'when', 'where', 'whip', 'whisper',
    'white', 'who', 'whole', 'whom', 'whose', 'why', 'wicked', 'wide', 'widow', 'width',
    'wife', 'wild', 'will', 'win', 'wind', 'window', 'wine', 'wing', 'wink', 'winner',
    'winter', 'wire', 'wisdom', 'wise', 'wish', 'witness', 'wolf', 'woman', 'wonder', 'wood',
    'wool', 'word', 'work', 'world', 'worry', 'worth', 'would', 'wrap', 'wreck', 'wrestle',
    'wrist', 'write', 'wrong', 'yard', 'year', 'yellow', 'you', 'young', 'youth', 'zebra',
    'zero', 'zone', 'zoo'
  ]
  
  const words = []
  for (let i = 0; i < 12; i++) {
    const randomIndex = Math.floor(Math.random() * wordList.length)
    words.push(wordList[randomIndex])
  }
  
  return words.join(' ')
}

// Validate if an address is a valid IPPAN address
export function validateAddress(address: string): boolean {
  if (!address.startsWith(IPPAN_PREFIX)) return false
  if (address.length !== ADDRESS_LENGTH) return false
  
  try {
    const hexPart = address.slice(1) // Remove the 'i' prefix
    // Check if it's valid hex and exactly 64 characters
    if (hexPart.length !== 64) return false
    if (!/^[0-9a-fA-F]{64}$/.test(hexPart)) return false
    return true
  } catch {
    return false
  }
}

// Derive address from seed phrase
export async function deriveAddressFromSeed(seedPhrase: string): Promise<string> {
  const seedData = new TextEncoder().encode(seedPhrase)
  const seedHash = await sha256(seedData)
  
  const combined = new Uint8Array(seedHash.length + 20) // 20 bytes for 'IPPAN_SEED_DERIVATION'
  combined.set(seedHash)
  const text = new TextEncoder().encode('IPPAN_SEED_DERIVATION')
  combined.set(text, seedHash.length)
  
  const publicKey = await sha256(combined)
  const publicKeyHex = Array.from(publicKey).map(b => b.toString(16).padStart(2, '0')).join('')
  
  return generateAddress(publicKeyHex)
}
