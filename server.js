const {
    RootMsg,
    Sm64JsMsg,
    SkinMsg,
    FlagMsg,
    AnnouncementMsg,
    ChatMsg,
    InitializationMsg,
    AuthorizedUserMsg,
    InitGameDataMsg,
    InitNewMarioStateMsg
} = require("./proto/mario_pb")

const got = require('got')
const jwt = require('jsonwebtoken')
const util = require('util')
const { v4: uuidv4 } = require('uuid')
const FileSync = require('lowdb/adapters/FileSync')
const zlib = require('zlib')
const deflate = util.promisify(zlib.deflate)
const inflate = util.promisify(zlib.inflate)
const port = 3080
const ws_port = 3000

const adminTokens = process.env.PRODUCTION ? process.env.ADMIN_TOKENS.split(":") : ["testAdminToken"]

const adapter = (process.env.PRODUCTION == 1) ? new FileSync('/tmp/data/db.json') : new FileSync('testdb.json')
const db = require('lowdb')(adapter)
db.defaults({ chats: [], adminCommands: [], ipList: [], accounts: {} }).write()

const gameMasterKey = process.env.PRODUCTION ? process.env.GAMEMASTER_KEY : "master"
if (gameMasterKey == undefined) throw "Error could not find Env var GAMEMASTER_KEY"

const standardLevels = require('./levelData').standardLevels

const allGames = {}
const publicLevelsToGameIds = {}
const socketIdsToGameIds = {}
let socketsInLobby = []

let masterSocket

const connectedIPs = {}
const stats = {}

let currentId = 0
const generateID = () => {
    if (++currentId > 4294967294) currentId = 0
    return currentId
}

const sendData = (bytes, socket) => { if (!socket.closed) socket.send(bytes, true) }

const broadcastData = (bytes, gameID) => {
    if (gameID == "lobbySockets") { // send to lobbySockets
        socketsInLobby.forEach(socket => { sendData(bytes, socket) })
    } else if (gameID) { // send to single game
        if (allGames[gameID]) Object.values(allGames[gameID].players).forEach(x => { sendData(bytes, x.socket) })
    } else { /// send to all games 
        Object.values(allGames).forEach(gameData => {
            Object.values(gameData.players).forEach(x => { sendData(bytes, x.socket) })
        })
    }
}

const initNewLevel = (level, public) => {
    const gameID = uuidv4()

    const flagPositions = standardLevels[level].flagStartPositions

    const newFlagData = new Array(flagPositions.length).fill(0).map((_, i) => {
        return {
            pos: [...flagPositions[i]],
            linkedToPlayer: false,
            atStartPosition: true,
            socketID: null,
            idleTimer: 0,
            heightBeforeFall: 20000
        }
    })

    allGames[gameID] = {
        players: [],
        flagData: newFlagData,
        level,
        public,
        inactiveCount: 0
    }

    return gameID
}


const unpackGameMasterData = async (bytes) => {
    const buffer = await (async () => {
        try {
            return await inflate(bytes)
        } catch (err) {
            console.log("Error with zlip inflate")
            return null
        }
    })()

    if (buffer == null) return

    const marioList = Sm64JsMsg.deserializeBinary(buffer).getGameDataMsg().getMarioList()
    const gameID = publicLevelsToGameIds[16] // castle grounds

    if (allGames[gameID] == undefined) return

    const valid_player_ids = marioList.map(marioProto => { return marioProto.getSocketid() })

    Object.entries(allGames[gameID].players).forEach(([socket_id, data]) => {
        if (!valid_player_ids.includes(parseInt(socket_id))) {  /// is not a valid player from game master
            if (Date.now() - data.joinTimeStamp > 2000) delete allGames[gameID].players[socket_id]
        }
    })
}


const processSkin = (socket_id, skinMsg) => {

    const gameID = socketIdsToGameIds[socket_id]
    if (gameID == undefined) return 

    if (allGames[gameID].players[socket_id] == undefined) return

    const skinData = skinMsg.getSkindata()

    allGames[gameID].players[socket_id].skinData = skinData
    allGames[gameID].players[socket_id].skinDataUpdated = true

}

const rejectPlayerName = (socket) => {
    const initGameDataMsg = new InitGameDataMsg()
    initGameDataMsg.setAccepted(false)
    const initializationMsg = new InitializationMsg()
    initializationMsg.setInitGameDataMsg(initGameDataMsg)
    const sm64jsMsg = new Sm64JsMsg()
    sm64jsMsg.setInitializationMsg(initializationMsg)
    const rootMsg = new RootMsg()
    rootMsg.setUncompressedSm64jsMsg(sm64jsMsg)
    sendData(rootMsg.serializeBinary(), socket)
}

const rejectAuthorization = (socket, status, rejectMessage) => {
    const rejectAuthorizationMsg = new AuthorizedUserMsg()
    if (rejectMessage) rejectAuthorizationMsg.setMessage(rejectMessage)
    rejectAuthorizationMsg.setStatus(status)
    const initializationMsg = new InitializationMsg()
    initializationMsg.setAuthorizedUserMsg(rejectAuthorizationMsg)
    const sm64jsMsg = new Sm64JsMsg()
    sm64jsMsg.setInitializationMsg(initializationMsg)
    const rootMsg = new RootMsg()
    rootMsg.setUncompressedSm64jsMsg(sm64jsMsg)
    sendData(rootMsg.serializeBinary(), socket)
    return false
}

const sanitizeChat = (string) => {
    string = string.substring(0, 200)
    return applyValidCharacters(string)
}

//Valid characters for usernames.
const validCharacters = new Set([
    'a', 'b', 'c', 'd', 'e', 'f', 'g',
    'h', 'i', 'j', 'k', 'l', 'm', 'n',
    'o', 'p', 'q', 'r', 's', 't', 'u',
    'v', 'w', 'y', 'x', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P',
    'Q', 'R', 'S', 'T', 'U', 'V', 'W',
    'Y', 'X', 'Z', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '0', '!',
    '@', '$', '^', '*', '(', ')', '{',
    '}', '[', ']', ';', ':', `'`, '"',
    `\\`, ',', '.', '/', '?', '🙄', '😫',
    '🤔', '🔥', '😌', '😍', '🤣', '❤️', '😭',
    '😂', '⭐', '✨', '🎄', '🎃', '🔺', '🔻',
    '🎄', '🍬', '🍭', '🍫', ' ',
    '-', '_', '=', '|', '<', '>', ':', "'"
]);


const applyValidCharacters = (str) => {
    return str.split('').filter(c => validCharacters.has(c)).join('');
}

const processAdminCommand = (msg, token, gameID) => {
    const parts = msg.split(' ')
    const command = parts[0].toUpperCase()
    const remainingParts = parts.slice(1)
    const args = remainingParts.join(" ")

    switch (command) {
        case "ANNOUNCEMENT":
            const announcementMsg = new AnnouncementMsg()
            announcementMsg.setMessage(args)
            announcementMsg.setTimer(300)
            const sm64jsMsg = new Sm64JsMsg()
            sm64jsMsg.setAnnouncementMsg(announcementMsg)
            const rootMsg = new RootMsg()
            rootMsg.setUncompressedSm64jsMsg(sm64jsMsg)
            broadcastData(rootMsg.serializeBinary(), gameID)
            break
        default:  return console.log("Unknown Admin Command: " + parts[0])
    }

    db.get('adminCommands').push({ token, gameID, timestampMs: Date.now(), command, args }).write()
}

const sendServerChatMsgToSocket = (socket, message) => {
    const chatMsg = new ChatMsg()
    chatMsg.setSocketid(socket.my_id)
    chatMsg.setMessage(message)
    chatMsg.setSender("Server")
    const sm64jsMsg = new Sm64JsMsg()
    sm64jsMsg.setChatMsg(chatMsg)
    const rootMsg = new RootMsg()
    rootMsg.setUncompressedSm64jsMsg(sm64jsMsg)
    sendData(rootMsg.serializeBinary(), socket)
}

const processChat = async (socket_id, sm64jsMsg) => {
    const chatMsg = sm64jsMsg.getChatMsg()
    const message = chatMsg.getMessage()

    const gameID = socketIdsToGameIds[socket_id]
    if (gameID == undefined) return 

    const playerData = allGames[gameID].players[socket_id]
    if (playerData == undefined) return

    /// Throttle chats by IP
    if (connectedIPs[playerData.socket.ip].chatCooldown > 10) {
        sendServerChatMsgToSocket(playerData.socket, "Chat message ignored: You have to wait longer between sending chat messages")
        return
    }

    const account = db.get('accounts.' + playerData.socket.accountID).value()
    if (account.muted) {
        if (account.tempBanTimestamp < Date.now()) {
            ///account.muted = false
            db.get('accounts.' + playerData.socket.accountID).assign({ muted: false }).write()
            //delete account.tempBanTimestamp
            db.unset('accounts.' + playerData.socket.accountID + '.tempBanTimestamp').write()
        } else { /// still muted
            sendServerChatMsgToSocket(playerData.socket, "Chat message ignored: Your account is muted, please contact a moderator")
            return
        }
    }

    if (message.length == 0) return

    const adminToken = chatMsg.getAdmintoken()
    const isAdmin = adminToken != null && adminTokens.includes(adminToken)

    if (message[0] == '/') {
        if (isAdmin) processAdminCommand(message.slice(1), adminToken, gameID)
        return
    }

    connectedIPs[playerData.socket.ip].chatCooldown += 3 // seconds

    /// record chat to DB
    db.get('chats').push({
        chatID: uuidv4(),
        accountID: playerData.socket.accountID,
        playerName: playerData.playerName,
        timestampMs: Date.now(),
        message
    }).write()

    const sanitizedChat = sanitizeChat(message)

    const request = "http://www.purgomalum.com/service/json?text=" + sanitizedChat

    try {
        const filteredMessage = JSON.parse((await got(request)).body).result

        chatMsg.setSocketid(socket_id)
        chatMsg.setMessage(filteredMessage)
        chatMsg.setSender(playerData.playerName)
        chatMsg.setIsadmin(isAdmin)

        const rootMsg = new RootMsg()
        rootMsg.setUncompressedSm64jsMsg(sm64jsMsg)
        broadcastData(rootMsg.serializeBinary(), gameID)

    } catch (e) {
        console.log(`Got error with profanity api: ${e}`)
    }

}

const processJoinGame = async (socket, msg) => {

    if (socketIdsToGameIds[socket.my_id] != undefined) return ///already initialized

    //// account has been authorized
    if (socket.accountID == undefined) return rejectPlayerName(socket)

    let name

    if (msg.getUseDiscordName()) {
        name = socket.discord.username
    } else {
        name = msg.getName()

        //// Verify custom name is allowed name 
        if (name.length < 3 || name.length > 14 || name.toUpperCase() == "SERVER") {
            return rejectPlayerName(socket)
        }

        const sanitizedName = sanitizeChat(name)
        if (sanitizedName != name) { return rejectPlayerName(socket) }

        const playerNameRequest = "http://www.purgomalum.com/service/json?text=" + sanitizedName

        try {
            const filteredPlayerName = JSON.parse((await got(playerNameRequest)).body).result
            if (sanitizedName != filteredPlayerName) { return rejectPlayerName(socket) }
        } catch (e) {
            console.log(`Got error with profanity api: ${e}`)
            return rejectPlayerName(socket)
        }
    }

    const level = msg.getLevel()

    if (level != 16) returnrejectPlayerName(socket)

    let gameID = publicLevelsToGameIds[level]

/*    if (level == 0) { /// custom game room
        gameID = msg.getGameId()
        if (allGames[gameID] == undefined) return rejectPlayerName(socket)
    } else {  /// normal server room
        if (standardLevels[level] == undefined) return rejectPlayerName(socket)
        gameID = publicLevelsToGameIds[level]
        if (allGames[gameID] == undefined) {  //// public room doesn't exist, create
            gameID = initNewLevel(level, true)
            publicLevelsToGameIds[level] = gameID
        }
    }*/

    allGames[gameID].inactiveCount = 0 /// some activity

    //Don't allow duplicate names in same room
    if (!msg.getUseDiscordName()) {
        const takenPlayerNames = Object.values(allGames[gameID].players).map(obj => obj.playerName)
        if (takenPlayerNames.includes(name)) return rejectPlayerName(socket)
    }

    db.get('accounts.' + socket.accountID).assign({ lastKnownPlayerName: name }).write()

    ////Success point - should initialize player
    allGames[gameID].players[socket.my_id] = {
        socket, /// also contains socket_id and ip
        playerName: name,
        joinTimeStamp: Date.now(),
        skinData: null
    }
    socketIdsToGameIds[socket.my_id] = gameID

    socketsInLobby = socketsInLobby.filter((lobbySocket) => { return lobbySocket != socket })


    /// send init mario data to game master
    const initNewMarioStateMsg = new InitNewMarioStateMsg()
    initNewMarioStateMsg.setSocketId(socket.my_id)
    const sm64jsMsg2 = new Sm64JsMsg()
    sm64jsMsg2.setInitNewMarioStateMsg(initNewMarioStateMsg)
    const rootMsg2 = new RootMsg()
    rootMsg2.setUncompressedSm64jsMsg(sm64jsMsg2)
    sendData(rootMsg2.serializeBinary(), masterSocket) 

    /// send accept join game to client
    const initGameDataMsg = new InitGameDataMsg()
    initGameDataMsg.setName(name)
    initGameDataMsg.setLevel(allGames[gameID].level)
    initGameDataMsg.setAccepted(true)
    initGameDataMsg.setSocketId(socket.my_id)
    const initializationMsg = new InitializationMsg()
    initializationMsg.setInitGameDataMsg(initGameDataMsg)
    const sm64jsMsg = new Sm64JsMsg()
    sm64jsMsg.setInitializationMsg(initializationMsg)
    const rootMsg = new RootMsg()
    rootMsg.setUncompressedSm64jsMsg(sm64jsMsg)
    sendData(rootMsg.serializeBinary(), socket)
}

const sendSkinsToSocket = (socket) => { 

    setTimeout(() => {
        const gameID = socketIdsToGameIds[socket.my_id]
        if (gameID == undefined || allGames[gameID] == undefined) {
            return  /// if they disconnect in this 500ms period
        }
        /// Send Skins
        Object.entries(allGames[gameID].players).filter(([_, data]) => data.skinData).forEach(([socket_id, data]) => {
            const skinMsg = new SkinMsg()
            skinMsg.setSocketid(socket_id)
            skinMsg.setSkindata(data.skinData)
            skinMsg.setPlayername(data.playerName)
            const sm64jsMsg = new Sm64JsMsg()
            sm64jsMsg.setSkinMsg(skinMsg)
            const rootMsg = new RootMsg()
            rootMsg.setUncompressedSm64jsMsg(sm64jsMsg)
            sendData(rootMsg.serializeBinary(), socket)
        })
    }, 500)

}
const sendSkinsIfUpdated = () => {

    Object.entries(allGames).forEach(([gameID, gameData]) => {
        /// Send Skins
        Object.entries(gameData.players).filter(([_, data]) => data.skinData && data.skinDataUpdated).forEach(([socket_id, data]) => {
            const skinMsg = new SkinMsg()
            skinMsg.setSocketid(socket_id)
            skinMsg.setSkindata(data.skinData)
            skinMsg.setPlayername(data.playerName)
            const sm64jsMsg = new Sm64JsMsg()
            sm64jsMsg.setSkinMsg(skinMsg)
            const rootMsg = new RootMsg()
            rootMsg.setUncompressedSm64jsMsg(sm64jsMsg)

            data.skinDataUpdated = false

            broadcastData(rootMsg.serializeBinary(), gameID)
        })
    })

}

const processBasicAttack = (attackerID, attackMsg) => {

    const gameID = socketIdsToGameIds[attackerID]
    if (gameID == undefined) return

    const playerData = allGames[gameID].players[attackerID]
    if (playerData == undefined) return

    /// redundant
    attackMsg.setAttackerSocketId(attackerID)

    const flagIndex = attackMsg.getFlagId()
    const targetId = attackMsg.getTargetSocketId()

    const theFlag = allGames[gameID].flagData[flagIndex]

    if (theFlag.linkedToPlayer && theFlag.socketID == targetId) {
        theFlag.linkedToPlayer = false
        theFlag.socketID = null
        theFlag.fallmode = true
        const newFlagLocation = playerData.decodedMario.getPosList()
        newFlagLocation[0] += ((Math.random() * 1000.0) - 500.0)
        newFlagLocation[1] += 600
        newFlagLocation[2] += ((Math.random() * 1000.0) - 500.0)
        theFlag.heightBeforeFall = newFlagLocation[1]
        theFlag.pos = [parseInt(newFlagLocation[0]), parseInt(newFlagLocation[1]), parseInt(newFlagLocation[2])]
    }

}

const processGrabFlagRequest = (socket_id, grabFlagMsg) => {

    const gameID = socketIdsToGameIds[socket_id]
    if (gameID == undefined) return

    const playerData = allGames[gameID].players[socket_id]
    if (playerData == undefined) return

    const i = grabFlagMsg.getFlagId()

    const theFlag = allGames[gameID].flagData[i]

    if (theFlag.linkedToPlayer) return

    const pos = grabFlagMsg.getPosList()

    const xDiff = pos[0] - theFlag.pos[0]
    const zDiff = pos[2] - theFlag.pos[2]

    const dist = Math.sqrt(xDiff * xDiff + zDiff * zDiff)
    if (dist < 50) {
        theFlag.linkedToPlayer = true
        theFlag.fallmode = false
        theFlag.atStartPosition = false
        theFlag.socketID = socket_id
        theFlag.idleTimer = 0
    }
}

const checkForFlag = (socket_id) => {

    Object.values(allGames).forEach(gameData => {
        gameData.flagData.forEach(flag => {
            if (flag.socketID == socket_id) {

                const playerData = gameData.players[socket_id]
                if (playerData == undefined) return

                flag.linkedToPlayer = false
                flag.socketID = null
                flag.fallmode = true
                const newFlagLocation = playerData.decodedMario.getPosList()
                newFlagLocation[1] += 100
                flag.heightBeforeFall = newFlagLocation[1]
                flag.pos = [parseInt(newFlagLocation[0]), parseInt(newFlagLocation[1]), parseInt(newFlagLocation[2])]
            }

        })
    })

}

const serverSideFlagUpdate = () => {

    Object.values(allGames).forEach(gameData => {
        gameData.flagData.forEach((flag, flagIndex) => {
            if (flag.fallmode) {
                if (flag.pos[1] > -10000) flag.pos[1] -= 2
            }

            if (!flag.linkedToPlayer && !flag.atStartPosition) {
                flag.idleTimer++
                if (flag.idleTimer > 3000) {

                    flag.pos = [...standardLevels[gameData.level].flagStartPositions[flagIndex]]
                    flag.fallmode = false
                    flag.atStartPosition = true
                    flag.idleTimer = 0
                }
            }
        })
    })

}



const processAccount = (socket, accountType) => {
    const account = db.get('accounts.' + socket.accountID).value()
    if (account) { /// account exists
        if (account.banned) {
            if (account.tempBanTimestamp) { // temp ban
                if (account.tempBanTimestamp > Date.now()) {  /// still temp banned
                    return rejectAuthorization(socket, 2, `Your account: ${socket.accountID} is temporaily banned, try again later`)
                } else { /// temp ban expired
                    ///account.banned = false
                    db.get('accounts.' + socket.accountID).assign({ banned: false }).write()
                    //delete account.tempBanTimestamp
                    db.unset('accounts.' + socket.accountID + '.tempBanTimestamp').write()
                }
            } else return rejectAuthorization(socket, 2, `Your account: ${socket.accountID} is banned, contact a moderator`)
        }
    } else {  /// account doesn't exist
        /// init new account
        db.set('accounts.' + socket.accountID, {
            type: accountType,
            banned: false,
            muted: false,
            banHistory: []
        }).write()
    }

    db.get('accounts.' + socket.accountID).assign({ lastLoginTime: Date.now() }).write()

    return true
}

const processAccessCode = async (socket, msg) => {

    const access_code = msg.getAccessCode()
    const type = msg.getType()

    if (access_code == undefined) return rejectAuthorization(socket, 2, "No Access Code Provided")

    if (access_code == gameMasterKey) {
        console.log("Game Master Auth Success!")
        masterSocket = socket
        publicLevelsToGameIds[16] = initNewLevel(16, true)
        return
    }

    if (process.env.PRODUCTION == 1) {

        if (type == "google") {
            const data = {
                client_id: process.env.GOOGLE_CLIENT_ID + ".apps.googleusercontent.com",
                client_secret: process.env.GOOGLE_CLIENT_SECRET,
                grant_type: 'authorization_code',
                redirect_uri: process.env.PRODUCTION_LOCAL ? 'http://localhost:9300' : 'https://sm64js.com',
                code: access_code
            }

            const result = await got.post('https://www.googleapis.com/oauth2/v4/token', {
                form: data,
                responseType: 'json'
            }).catch((err) => { })

            if (result == undefined || result.body == undefined) return rejectAuthorization(socket, 0, "Failed to get Google Account Authorization Token")

            const decoded = jwt.decode(result.body.id_token)

            if (decoded == undefined || decoded.sub == undefined) return rejectAuthorization(socket, 0, "Failed to get Google Account Info")

            socket.accountID = "google-" + decoded.sub

            const success = processAccount(socket, "google")
            if (!success) return

        }
        else if (type == "discord") {

            const data = {
                client_id: process.env.DISCORD_CLIENT_ID,
                client_secret: process.env.DISCORD_CLIENT_SECRET,
                grant_type: 'authorization_code',
                redirect_uri: process.env.PRODUCTION_LOCAL ? 'http://localhost:9300' : 'https://sm64js.com',
                code: access_code,
                scope: 'guilds',
            }

            const result = await got.post('https://discord.com/api/oauth2/token', {
                form: data,
                responseType: 'json'
            }).catch((err) => { })

            if (result == undefined) return rejectAuthorization(socket, 0, "Failed to get Discord Account Authorization Token")

            const { access_token, token_type } = result.body

            const userData = await got('https://discord.com/api/users/@me', {
                headers: { authorization: `${token_type} ${access_token}` },
                responseType: 'json'
            }).catch((err) => { })

            if (userData == undefined || userData.body == undefined) return rejectAuthorization(socket, 0, "Failed to get Discord Account Info")

            socket.accountID = "discord-" + userData.body.id
            socket.discord = { userData: userData.body, access_token }
            socket.discord.username = userData.body.username + "#" + userData.body.discriminator

            const success = processAccount(socket, "discord")
            if (!success) return

        } else {
            return rejectAuthorization(socket, 2, "Unknown Account Type")
        }

    } else {  /// Testing locally
        socket.accountID = "discord-12356789"
        socket.discord = { username: "SnuffysasaTest#1234" }
    }


    const authorizedUserMsg = new AuthorizedUserMsg()
    if (socket.discord) {
        authorizedUserMsg.setUsername(socket.discord.username)
    }
    authorizedUserMsg.setStatus(1)
    const initializationMsg = new InitializationMsg()
    initializationMsg.setAuthorizedUserMsg(authorizedUserMsg)
    const sm64jsMsg = new Sm64JsMsg()
    sm64jsMsg.setInitializationMsg(initializationMsg)
    const rootMsg = new RootMsg()
    rootMsg.setUncompressedSm64jsMsg(sm64jsMsg)
    sendData(rootMsg.serializeBinary(), socket)
}




/// 30 times per second
setInterval(async () => {

    serverSideFlagUpdate()

/*   /// leaving this section commented out for reference of flag data
    Object.entries(allGames).forEach(async ([gameID, gameData]) => {
        const sm64jsMsg = new Sm64JsMsg()
        const mariolist = Object.values(gameData.players).filter(data => data.decodedMario).map(data => data.decodedMario)
        const mariolistproto = new MarioListMsg()

        mariolistproto.setMarioList(mariolist)

        const flagProtoList = []

        for (let i = 0; i < gameData.flagData.length; i++) {
            const theFlag = gameData.flagData[i]
            const flagmsg = new FlagMsg()
            flagmsg.setLinkedtoplayer(theFlag.linkedToPlayer)
            if (theFlag.linkedToPlayer) flagmsg.setSocketid(theFlag.socketID)
            else {
                flagmsg.setPosList(theFlag.pos)
                flagmsg.setHeightBeforeFall(theFlag.heightBeforeFall)
            }
            flagProtoList.push(flagmsg)
        }

        mariolistproto.setFlagList(flagProtoList)

        sm64jsMsg.setListMsg(mariolistproto)
        const bytes = sm64jsMsg.serializeBinary()
        const compressedBytes = await deflate(bytes)
        const rootMsg = new RootMsg()
        rootMsg.setCompressedSm64jsMsg(compressedBytes)
        //sendData(rootMsg.serializeBinary(), masterSocket)
        //broadcastData(rootMsg.serializeBinary(), gameID)   dont send this way
    })
*/

}, 31)  /// 31 seems to be sweet spot

/// Every 33 frames / once per second
setInterval(() => {
    //sendValidUpdate()

    //chat cooldown
    Object.values(connectedIPs).forEach(data => {
        if (data.chatCooldown > 0) data.chatCooldown--
    })
}, 1000)

/// Every 10 seconds - send skins
setInterval(() => {

    sendSkinsIfUpdated()

}, 10000)


/// Every 5 minutes - delete inactive games
setInterval(() => {

    Object.entries(allGames).forEach(([gameID, gameData]) => {

        if (Object.values(gameData.players).length == 0) { //inactive game
            gameData.inactiveCount++

            if (gameData.inactiveCount >= 5) {
                /// delete game
                delete allGames[gameID]
                delete publicLevelsToGameIds[gameData.level]
            }

        }

    })

}, 300000)

//Every 1 day - Auto Delete Old chat entries
setInterval(() => {
    const threeDaysAgo = Date.now() - (86400000 * 3)
    db.get('chats').remove((entry) => {
        if (entry.timestampMs < threeDaysAgo) return true
    }).write()
}, 86400000) //1 Days


require('uWebSockets.js').App().ws('/*', {

    upgrade: async (res, req, context) => { // a request was made to open websocket, res req have all the properties for the request, cookies etc

        // add code here to determine if ws request should be accepted or denied
        // can deny request with "return res.writeStatus('401').end()" see issue #367

        const ip = req.getHeader('x-forwarded-for')

        if (connectedIPs[ip]) {
            if (Object.keys(connectedIPs[ip].socketIDs).length >= 4) return res.writeStatus('403').end()
        }

        const key = req.getHeader('sec-websocket-key')
        const protocol = req.getHeader('sec-websocket-protocol')
        const extensions = req.getHeader('sec-websocket-extensions')

        res.onAborted(() => {})

        if (process.env.PRODUCTION && process.env.PRODUCTION_LOCAL != 1) {

            try {

                //console.log("someone trying to connect: " + ip)

                ///// check CORS
                if (process.env.ENFORCE_CORS_ON_WS == 1) {
                    let originHeader = req.getHeader('origin')
                    const url = new URL(originHeader)
                    const domainStr = url.hostname.substring(url.hostname.length - 11, url.hostname.length)
                    if (domainStr != ".sm64js.com" && url.hostname != "sm64js.com") return res.writeStatus('418').end()
                }

                //// Going to remove
                const ipStatus = db.get('ipList').find({ ip }).value()

                if (ipStatus == undefined) {

                    if (process.env.USE_VPN) {

                        //console.log("trying to hit vpn api")
                        const vpnCheckRequest = `http://v2.api.iphub.info/ip/${ip}`
                        const initApiReponse = await got(vpnCheckRequest, {
                            headers: { 'X-Key': process.env.VPN_API_KEY }
                        })
                        const response = JSON.parse(initApiReponse.body)

                        if (response.block == undefined) {
                            console.log("iphub reponse invalid")
                            return res.writeStatus('500').end()
                        }

                        if (response.block == 1) {
                            db.get('ipList').push({ ip, value: 'BANNED', reason: 'AutoVPN' }).write()
                            // console.log("Adding new VPN BAD IP " + ip)
                            return res.writeStatus('403').end()
                        } else {
                            //console.log("Adding new Legit IP")
                            db.get('ipList').push({ ip, value: 'ALLOWED' }).write()
                        }
                    }

                } else if (ipStatus.value == "BANNED") {  /// BANNED or NOT ALLOWED IP - Going to remove
                    return res.writeStatus('403').end()
                }

            } catch (e) {
                console.log(`Got error with upgrading to websocket: ${e}`)
                return res.writeStatus('500').end()
            }

        }
        
        res.upgrade( // upgrade to websocket
            { ip }, // 1st argument sets which properties to pass to the ws object, in this case ip address
            key,
            protocol,
            extensions, // these 3 headers are used to setup the websocket
            context // also used to setup the websocket
        )


    },

    open: async (socket) => {
        socket.my_id = generateID()

        if (connectedIPs[socket.ip] == undefined)
            connectedIPs[socket.ip] = { socketIDs: {}, chatCooldown: 15 }

        connectedIPs[socket.ip].socketIDs[socket.my_id] = 1

        socketsInLobby.push(socket)
    },

    message: async (socket, bytes) => {

        try {
            let sm64jsMsg
            const rootMsg = RootMsg.deserializeBinary(bytes)

            switch (rootMsg.getMessageCase()) {
                case RootMsg.MessageCase.COMPRESSED_SM64JS_MSG:
                    if (socket == masterSocket) {
                        unpackGameMasterData(rootMsg.getCompressedSm64jsMsg())
                        broadcastData(bytes)  /// send allMarioList to all sockets but the gameMaster
                    } else {
                        console.log("should not be receiving this message")
                    }
                    break
                case RootMsg.MessageCase.UNCOMPRESSED_SM64JS_MSG:

                    sm64jsMsg = rootMsg.getUncompressedSm64jsMsg()
                    switch (sm64jsMsg.getMessageCase()) {
                        case Sm64JsMsg.MessageCase.MARIO_MSG:
                            //if (socketIdsToGameIds[socket.my_id] == undefined) return 
                            //processPlayerData(socket.my_id, sm64jsMsg.getMarioMsg()); break
                        case Sm64JsMsg.MessageCase.CONTROLLER_MSG:
                            if (masterSocket) sendData(bytes, masterSocket)
                            break
                        case Sm64JsMsg.MessageCase.PING_MSG:
                            sendData(bytes, socket); break
                        case Sm64JsMsg.MessageCase.ATTACK_MSG:
                            if (socketIdsToGameIds[socket.my_id] == undefined) return 
                            processBasicAttack(socket.my_id, sm64jsMsg.getAttackMsg()); break
                        case Sm64JsMsg.MessageCase.GRAB_MSG:
                            if (socketIdsToGameIds[socket.my_id] == undefined) return 
                            processGrabFlagRequest(socket.my_id, sm64jsMsg.getGrabMsg()); break
                        case Sm64JsMsg.MessageCase.CHAT_MSG:
                            if (socketIdsToGameIds[socket.my_id] == undefined) return 
                            processChat(socket.my_id, sm64jsMsg); break
                        case Sm64JsMsg.MessageCase.SKIN_MSG:
                            if (socketIdsToGameIds[socket.my_id] == undefined) return 
                            processSkin(socket.my_id, sm64jsMsg.getSkinMsg()); break
                        case Sm64JsMsg.MessageCase.INITIALIZATION_MSG:
                            const initializationMsg = sm64jsMsg.getInitializationMsg()
                            switch (initializationMsg.getMessageCase()) {
                                case InitializationMsg.MessageCase.ACCESS_CODE_MSG:
                                    processAccessCode(socket, initializationMsg.getAccessCodeMsg()); break
                                case InitializationMsg.MessageCase.JOIN_GAME_MSG:
                                    processJoinGame(socket, initializationMsg.getJoinGameMsg()); break
                                case InitializationMsg.MessageCase.REQUEST_COSMETICS_MSG:
                                    sendSkinsToSocket(socket); break
                                default: throw "unknown case for initialization proto message"
                            }
                            break
                        default: throw "unknown case for uncompressed proto message"
                    }
                    break
                case RootMsg.MessageCase.MESSAGE_NOT_SET:
                default:
                    if (rootMsg.getMessageCase() != 0)
                        throw new Error(`unhandled case in switch expression: ${rootMsg.getMessageCase()}`)
            }


        } catch (err) { console.log(err) }
    },

    close: (socket) => {
        socket.closed = true
        checkForFlag(socket.my_id)
        delete connectedIPs[socket.ip].socketIDs[socket.my_id]

        socketsInLobby = socketsInLobby.filter((lobbySocket) => { return lobbySocket != socket })

        const gameID = socketIdsToGameIds[socket.my_id]
        if (gameID) {
            delete allGames[gameID].players[socket.my_id]
            delete socketIdsToGameIds[socket.my_id]
        }
    }

}).listen(ws_port, () => { console.log("Starting websocket server " + ws_port) })

//// Express Static serving
const express = require('express')
const app = express()
const server = require('http').Server(app)

app.use(express.urlencoded({ extended: true }))
app.use(express.json())

server.listen(port, () => { console.log('Starting Express server for http requests ' + port) })


////// Admin Commands
app.post("/validateAdminToken", function (req, res) {
    let valid = true
    const token = req.body.token
    if (!adminTokens.includes(token)) valid = false

    res.send({ "valid": valid, "token": token })
})

app.get('/accountList', (req, res) => { ///query params: token, accountID

    const token = req.query.token
    if (!adminTokens.includes(token)) return res.status(401).send({ error: 'Invalid Admin Token' })

    const jsonResult = []
    const accounts = db.get('accounts').value()
    Object.entries(accounts).forEach(([accountID, data]) => {
        jsonResult.push({ accountID, lastLoginTime: data.lastLoginTime, lastKnownPlayerName: data.lastKnownPlayerName })
    })
    jsonResult.sort((a, b) => { return b.lastLoginTime - a.lastLoginTime })
    return res.send(jsonResult)
})


app.get('/accountLookup', (req, res) => { ///query params: token, accountID

    const token = req.query.token
    if (!adminTokens.includes(token)) return res.status(401).send({ error: 'Invalid Admin Token' })

    const account = db.get('accounts.' + req.query.accountID).value()
    if (account) {
        account.accountID = req.query.accountID
        return res.send(account)
    } else {
        return res.send({ error: "Account ID not found" })
    }

})

app.post('/manageAccount', (req, res) => { ///query params: token, accountID, ban, mute, comments, modName, durationInHours

    const token = req.query.token
    const { comments, modName, durationInHours, accountID, ban, mute } = req.body
    if (!adminTokens.includes(token)) return res.status(401).send({ error: 'Invalid Admin Token' })

    if (comments == undefined || comments == "")
        return res.send({ error: "Missing Param 'comments': You must include comments about the account status update" })
    if (modName == undefined || modName == "")
        return res.send({ error: "Missing Param 'modName': You must include the moderator name (your name), nickname is fine" })

    if (ban && mute) return res.send({ error: "You cannot ban and mute someone, try again with just ban" })

    if (!ban && !mute && durationInHours != undefined)
        return res.send({ error: "Invalid request: You can not include a duration with neither ban nor mute set" })

    const account = db.get('accounts.' + accountID).value()
    if (account) {

        const currentTimeStamp = Date.now()

        if (account.banned == ban && account.muted == mute)
            return res.send({ error: "account is already in the requested state, no changes have been made", account }) 

        let tempBanTimestamp
        if (durationInHours != 0) {
            if (Number(durationInHours) == NaN) return res.send({ error: "Invalid param durationInHours: resulted in NaN" })
            else tempBanTimestamp = currentTimeStamp + (Number(durationInHours) * 3600000)
        }

        db.get('adminCommands').push({ token, timestampMs: currentTimeStamp, command: 'manageAccount', args: [accountID] }).write()

        db.get('accounts.' + accountID + '.banHistory').push({
            timestamp: currentTimeStamp,
            ban,
            mute,
            comments,
            modName,
            durationInHours
        }).write()

        db.get('accounts.' + accountID).assign({ banned: ban, muted: mute, tempBanTimestamp }).write()

        if (account.banned) { ///disconnect current session
            Object.values(allGames).forEach(gameData => {
                Object.values(gameData.players).forEach(data => {
                    if (data.socket.accountID == accountID) data.socket.close()
                })
            })
        }

        return res.send({ message: "Successfully updated account", account })
    } else {
        return res.send({ error: "Account ID not found" })
    }

})


app.get('/chatLog', (req, res) => { ///query params token, timestamp, range, playerName, accountID

    const token = req.query.token
    if (!adminTokens.includes(token)) return res.status(401).send({ error: 'Invalid Admin Token' })

    const timestamp = (req.query.timestamp) ? parseInt(req.query.timestamp) * 1000 : Date.now()
    const range = parseInt(req.query.range ? req.query.range : 60) * 1000

    const jsonResult = []

    const dbQuery = {}
    if (req.query.accountID) dbQuery.accountID = req.query.accountID
    if (req.query.playerName) dbQuery.playerName = req.query.playerName

    db.get('chats').filter(dbQuery).filter(entry => {
        return entry.timestampMs >= timestamp - range && entry.timestampMs <= timestamp + range
    }).forEach((entry) => {
        jsonResult.push(entry)
    }).value()
        
    return res.send(jsonResult)

})

app.get('/adminLog', (req, res) => { ///query params token, 

    const token = req.query.token

    if (token != process.env.SENIOR_ADMIN_KEY) return res.status(401).send('Invalid Senior Admin Token')

    let stringResult = ""

    db.get('adminCommands').forEach((entry) => {
        stringResult += JSON.stringify(entry)
        stringResult += '<br />'
    }).value()

    return res.send(stringResult)

})

app.get('/createGame', (req, res) => {
    res.sendFile(__dirname + '/createGameForm.html')
})

app.post('/createNewGame', (req, res) => {

    const level = parseInt(req.body.level)

    if (standardLevels[level] == undefined) return res.status(401).send('Invalid Level/Map ID')

    const gameID = initNewLevel(level, false)

    return res.send(`Invite Link: <a href="https://sm64js.com/?gameID=${gameID}">https://sm64js.com/?gameID=${gameID}</a>`)

})

