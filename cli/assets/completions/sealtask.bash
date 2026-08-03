_sealtask() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="sealtask"
                ;;
            sealtask,activity)
                cmd="sealtask__subcmd__activity"
                ;;
            sealtask,agents)
                cmd="sealtask__subcmd__agents"
                ;;
            sealtask,auth)
                cmd="sealtask__subcmd__auth"
                ;;
            sealtask,batch)
                cmd="sealtask__subcmd__batch"
                ;;
            sealtask,browse)
                cmd="sealtask__subcmd__browse"
                ;;
            sealtask,cache)
                cmd="sealtask__subcmd__cache"
                ;;
            sealtask,comments)
                cmd="sealtask__subcmd__comments"
                ;;
            sealtask,completion)
                cmd="sealtask__subcmd__completion"
                ;;
            sealtask,config)
                cmd="sealtask__subcmd__config"
                ;;
            sealtask,doctor)
                cmd="sealtask__subcmd__doctor"
                ;;
            sealtask,help)
                cmd="sealtask__subcmd__help"
                ;;
            sealtask,info)
                cmd="sealtask__subcmd__info"
                ;;
            sealtask,inspect)
                cmd="sealtask__subcmd__inspect"
                ;;
            sealtask,lists)
                cmd="sealtask__subcmd__projects"
                ;;
            sealtask,man)
                cmd="sealtask__subcmd__man"
                ;;
            sealtask,me)
                cmd="sealtask__subcmd__me"
                ;;
            sealtask,notes)
                cmd="sealtask__subcmd__notes"
                ;;
            sealtask,pick)
                cmd="sealtask__subcmd__pick"
                ;;
            sealtask,profile)
                cmd="sealtask__subcmd__profile"
                ;;
            sealtask,projects)
                cmd="sealtask__subcmd__projects"
                ;;
            sealtask,schema)
                cmd="sealtask__subcmd__schema"
                ;;
            sealtask,stats)
                cmd="sealtask__subcmd__stats"
                ;;
            sealtask,tasks)
                cmd="sealtask__subcmd__tasks"
                ;;
            sealtask__subcmd__activity,follow)
                cmd="sealtask__subcmd__activity__subcmd__follow"
                ;;
            sealtask__subcmd__activity,help)
                cmd="sealtask__subcmd__activity__subcmd__help"
                ;;
            sealtask__subcmd__activity__subcmd__help,follow)
                cmd="sealtask__subcmd__activity__subcmd__help__subcmd__follow"
                ;;
            sealtask__subcmd__activity__subcmd__help,help)
                cmd="sealtask__subcmd__activity__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__agents,approve)
                cmd="sealtask__subcmd__agents__subcmd__approve"
                ;;
            sealtask__subcmd__agents,assign)
                cmd="sealtask__subcmd__agents__subcmd__assign"
                ;;
            sealtask__subcmd__agents,help)
                cmd="sealtask__subcmd__agents__subcmd__help"
                ;;
            sealtask__subcmd__agents,list)
                cmd="sealtask__subcmd__agents__subcmd__list"
                ;;
            sealtask__subcmd__agents,register)
                cmd="sealtask__subcmd__agents__subcmd__register"
                ;;
            sealtask__subcmd__agents,revoke)
                cmd="sealtask__subcmd__agents__subcmd__revoke"
                ;;
            sealtask__subcmd__agents,unassign)
                cmd="sealtask__subcmd__agents__subcmd__unassign"
                ;;
            sealtask__subcmd__agents__subcmd__help,approve)
                cmd="sealtask__subcmd__agents__subcmd__help__subcmd__approve"
                ;;
            sealtask__subcmd__agents__subcmd__help,assign)
                cmd="sealtask__subcmd__agents__subcmd__help__subcmd__assign"
                ;;
            sealtask__subcmd__agents__subcmd__help,help)
                cmd="sealtask__subcmd__agents__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__agents__subcmd__help,list)
                cmd="sealtask__subcmd__agents__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__agents__subcmd__help,register)
                cmd="sealtask__subcmd__agents__subcmd__help__subcmd__register"
                ;;
            sealtask__subcmd__agents__subcmd__help,revoke)
                cmd="sealtask__subcmd__agents__subcmd__help__subcmd__revoke"
                ;;
            sealtask__subcmd__agents__subcmd__help,unassign)
                cmd="sealtask__subcmd__agents__subcmd__help__subcmd__unassign"
                ;;
            sealtask__subcmd__auth,help)
                cmd="sealtask__subcmd__auth__subcmd__help"
                ;;
            sealtask__subcmd__auth,keychain)
                cmd="sealtask__subcmd__auth__subcmd__keychain"
                ;;
            sealtask__subcmd__auth,lock)
                cmd="sealtask__subcmd__auth__subcmd__lock"
                ;;
            sealtask__subcmd__auth,login)
                cmd="sealtask__subcmd__auth__subcmd__login"
                ;;
            sealtask__subcmd__auth,logout)
                cmd="sealtask__subcmd__auth__subcmd__logout"
                ;;
            sealtask__subcmd__auth,status)
                cmd="sealtask__subcmd__auth__subcmd__status"
                ;;
            sealtask__subcmd__auth,unlock)
                cmd="sealtask__subcmd__auth__subcmd__unlock"
                ;;
            sealtask__subcmd__auth__subcmd__help,help)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__auth__subcmd__help,keychain)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__keychain"
                ;;
            sealtask__subcmd__auth__subcmd__help,lock)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__lock"
                ;;
            sealtask__subcmd__auth__subcmd__help,login)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__login"
                ;;
            sealtask__subcmd__auth__subcmd__help,logout)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__logout"
                ;;
            sealtask__subcmd__auth__subcmd__help,status)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__status"
                ;;
            sealtask__subcmd__auth__subcmd__help,unlock)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__unlock"
                ;;
            sealtask__subcmd__auth__subcmd__help__subcmd__keychain,clear)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__keychain__subcmd__clear"
                ;;
            sealtask__subcmd__auth__subcmd__help__subcmd__keychain,store)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__keychain__subcmd__store"
                ;;
            sealtask__subcmd__auth__subcmd__keychain,clear)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__clear"
                ;;
            sealtask__subcmd__auth__subcmd__keychain,help)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__help"
                ;;
            sealtask__subcmd__auth__subcmd__keychain,store)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__store"
                ;;
            sealtask__subcmd__auth__subcmd__keychain__subcmd__help,clear)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__clear"
                ;;
            sealtask__subcmd__auth__subcmd__keychain__subcmd__help,help)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__auth__subcmd__keychain__subcmd__help,store)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__store"
                ;;
            sealtask__subcmd__batch,help)
                cmd="sealtask__subcmd__batch__subcmd__help"
                ;;
            sealtask__subcmd__batch,run)
                cmd="sealtask__subcmd__batch__subcmd__run"
                ;;
            sealtask__subcmd__batch__subcmd__help,help)
                cmd="sealtask__subcmd__batch__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__batch__subcmd__help,run)
                cmd="sealtask__subcmd__batch__subcmd__help__subcmd__run"
                ;;
            sealtask__subcmd__cache,clear)
                cmd="sealtask__subcmd__cache__subcmd__clear"
                ;;
            sealtask__subcmd__cache,help)
                cmd="sealtask__subcmd__cache__subcmd__help"
                ;;
            sealtask__subcmd__cache,status)
                cmd="sealtask__subcmd__cache__subcmd__status"
                ;;
            sealtask__subcmd__cache,verify)
                cmd="sealtask__subcmd__cache__subcmd__verify"
                ;;
            sealtask__subcmd__cache__subcmd__help,clear)
                cmd="sealtask__subcmd__cache__subcmd__help__subcmd__clear"
                ;;
            sealtask__subcmd__cache__subcmd__help,help)
                cmd="sealtask__subcmd__cache__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__cache__subcmd__help,status)
                cmd="sealtask__subcmd__cache__subcmd__help__subcmd__status"
                ;;
            sealtask__subcmd__cache__subcmd__help,verify)
                cmd="sealtask__subcmd__cache__subcmd__help__subcmd__verify"
                ;;
            sealtask__subcmd__comments,create)
                cmd="sealtask__subcmd__comments__subcmd__create"
                ;;
            sealtask__subcmd__comments,delete)
                cmd="sealtask__subcmd__comments__subcmd__delete"
                ;;
            sealtask__subcmd__comments,help)
                cmd="sealtask__subcmd__comments__subcmd__help"
                ;;
            sealtask__subcmd__comments,list)
                cmd="sealtask__subcmd__comments__subcmd__list"
                ;;
            sealtask__subcmd__comments,update)
                cmd="sealtask__subcmd__comments__subcmd__update"
                ;;
            sealtask__subcmd__comments__subcmd__help,create)
                cmd="sealtask__subcmd__comments__subcmd__help__subcmd__create"
                ;;
            sealtask__subcmd__comments__subcmd__help,delete)
                cmd="sealtask__subcmd__comments__subcmd__help__subcmd__delete"
                ;;
            sealtask__subcmd__comments__subcmd__help,help)
                cmd="sealtask__subcmd__comments__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__comments__subcmd__help,list)
                cmd="sealtask__subcmd__comments__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__comments__subcmd__help,update)
                cmd="sealtask__subcmd__comments__subcmd__help__subcmd__update"
                ;;
            sealtask__subcmd__config,help)
                cmd="sealtask__subcmd__config__subcmd__help"
                ;;
            sealtask__subcmd__config,show)
                cmd="sealtask__subcmd__config__subcmd__show"
                ;;
            sealtask__subcmd__config__subcmd__help,help)
                cmd="sealtask__subcmd__config__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__config__subcmd__help,show)
                cmd="sealtask__subcmd__config__subcmd__help__subcmd__show"
                ;;
            sealtask__subcmd__help,activity)
                cmd="sealtask__subcmd__help__subcmd__activity"
                ;;
            sealtask__subcmd__help,agents)
                cmd="sealtask__subcmd__help__subcmd__agents"
                ;;
            sealtask__subcmd__help,auth)
                cmd="sealtask__subcmd__help__subcmd__auth"
                ;;
            sealtask__subcmd__help,batch)
                cmd="sealtask__subcmd__help__subcmd__batch"
                ;;
            sealtask__subcmd__help,browse)
                cmd="sealtask__subcmd__help__subcmd__browse"
                ;;
            sealtask__subcmd__help,cache)
                cmd="sealtask__subcmd__help__subcmd__cache"
                ;;
            sealtask__subcmd__help,comments)
                cmd="sealtask__subcmd__help__subcmd__comments"
                ;;
            sealtask__subcmd__help,completion)
                cmd="sealtask__subcmd__help__subcmd__completion"
                ;;
            sealtask__subcmd__help,config)
                cmd="sealtask__subcmd__help__subcmd__config"
                ;;
            sealtask__subcmd__help,doctor)
                cmd="sealtask__subcmd__help__subcmd__doctor"
                ;;
            sealtask__subcmd__help,help)
                cmd="sealtask__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__help,info)
                cmd="sealtask__subcmd__help__subcmd__info"
                ;;
            sealtask__subcmd__help,inspect)
                cmd="sealtask__subcmd__help__subcmd__inspect"
                ;;
            sealtask__subcmd__help,man)
                cmd="sealtask__subcmd__help__subcmd__man"
                ;;
            sealtask__subcmd__help,me)
                cmd="sealtask__subcmd__help__subcmd__me"
                ;;
            sealtask__subcmd__help,notes)
                cmd="sealtask__subcmd__help__subcmd__notes"
                ;;
            sealtask__subcmd__help,pick)
                cmd="sealtask__subcmd__help__subcmd__pick"
                ;;
            sealtask__subcmd__help,profile)
                cmd="sealtask__subcmd__help__subcmd__profile"
                ;;
            sealtask__subcmd__help,projects)
                cmd="sealtask__subcmd__help__subcmd__projects"
                ;;
            sealtask__subcmd__help,schema)
                cmd="sealtask__subcmd__help__subcmd__schema"
                ;;
            sealtask__subcmd__help,stats)
                cmd="sealtask__subcmd__help__subcmd__stats"
                ;;
            sealtask__subcmd__help,tasks)
                cmd="sealtask__subcmd__help__subcmd__tasks"
                ;;
            sealtask__subcmd__help__subcmd__activity,follow)
                cmd="sealtask__subcmd__help__subcmd__activity__subcmd__follow"
                ;;
            sealtask__subcmd__help__subcmd__agents,approve)
                cmd="sealtask__subcmd__help__subcmd__agents__subcmd__approve"
                ;;
            sealtask__subcmd__help__subcmd__agents,assign)
                cmd="sealtask__subcmd__help__subcmd__agents__subcmd__assign"
                ;;
            sealtask__subcmd__help__subcmd__agents,list)
                cmd="sealtask__subcmd__help__subcmd__agents__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__agents,register)
                cmd="sealtask__subcmd__help__subcmd__agents__subcmd__register"
                ;;
            sealtask__subcmd__help__subcmd__agents,revoke)
                cmd="sealtask__subcmd__help__subcmd__agents__subcmd__revoke"
                ;;
            sealtask__subcmd__help__subcmd__agents,unassign)
                cmd="sealtask__subcmd__help__subcmd__agents__subcmd__unassign"
                ;;
            sealtask__subcmd__help__subcmd__auth,keychain)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__keychain"
                ;;
            sealtask__subcmd__help__subcmd__auth,lock)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__lock"
                ;;
            sealtask__subcmd__help__subcmd__auth,login)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__login"
                ;;
            sealtask__subcmd__help__subcmd__auth,logout)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__logout"
                ;;
            sealtask__subcmd__help__subcmd__auth,status)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__status"
                ;;
            sealtask__subcmd__help__subcmd__auth,unlock)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__unlock"
                ;;
            sealtask__subcmd__help__subcmd__auth__subcmd__keychain,clear)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__keychain__subcmd__clear"
                ;;
            sealtask__subcmd__help__subcmd__auth__subcmd__keychain,store)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__keychain__subcmd__store"
                ;;
            sealtask__subcmd__help__subcmd__batch,run)
                cmd="sealtask__subcmd__help__subcmd__batch__subcmd__run"
                ;;
            sealtask__subcmd__help__subcmd__cache,clear)
                cmd="sealtask__subcmd__help__subcmd__cache__subcmd__clear"
                ;;
            sealtask__subcmd__help__subcmd__cache,status)
                cmd="sealtask__subcmd__help__subcmd__cache__subcmd__status"
                ;;
            sealtask__subcmd__help__subcmd__cache,verify)
                cmd="sealtask__subcmd__help__subcmd__cache__subcmd__verify"
                ;;
            sealtask__subcmd__help__subcmd__comments,create)
                cmd="sealtask__subcmd__help__subcmd__comments__subcmd__create"
                ;;
            sealtask__subcmd__help__subcmd__comments,delete)
                cmd="sealtask__subcmd__help__subcmd__comments__subcmd__delete"
                ;;
            sealtask__subcmd__help__subcmd__comments,list)
                cmd="sealtask__subcmd__help__subcmd__comments__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__comments,update)
                cmd="sealtask__subcmd__help__subcmd__comments__subcmd__update"
                ;;
            sealtask__subcmd__help__subcmd__config,show)
                cmd="sealtask__subcmd__help__subcmd__config__subcmd__show"
                ;;
            sealtask__subcmd__help__subcmd__notes,create)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__create"
                ;;
            sealtask__subcmd__help__subcmd__notes,delete)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__delete"
                ;;
            sealtask__subcmd__help__subcmd__notes,edit)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__edit"
                ;;
            sealtask__subcmd__help__subcmd__notes,get)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__get"
                ;;
            sealtask__subcmd__help__subcmd__notes,list)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__notes,update)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__update"
                ;;
            sealtask__subcmd__help__subcmd__pick,project)
                cmd="sealtask__subcmd__help__subcmd__pick__subcmd__project"
                ;;
            sealtask__subcmd__help__subcmd__pick,task)
                cmd="sealtask__subcmd__help__subcmd__pick__subcmd__task"
                ;;
            sealtask__subcmd__help__subcmd__profile,list)
                cmd="sealtask__subcmd__help__subcmd__profile__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__profile,use)
                cmd="sealtask__subcmd__help__subcmd__profile__subcmd__use"
                ;;
            sealtask__subcmd__help__subcmd__projects,archive)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__archive"
                ;;
            sealtask__subcmd__help__subcmd__projects,audit)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__audit"
                ;;
            sealtask__subcmd__help__subcmd__projects,clear)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__clear"
                ;;
            sealtask__subcmd__help__subcmd__projects,current)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__current"
                ;;
            sealtask__subcmd__help__subcmd__projects,get)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__get"
                ;;
            sealtask__subcmd__help__subcmd__projects,list)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__projects,sections)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__sections"
                ;;
            sealtask__subcmd__help__subcmd__projects,unarchive)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__unarchive"
                ;;
            sealtask__subcmd__help__subcmd__projects__subcmd__sections,list)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__sections__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__tasks,archive)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__archive"
                ;;
            sealtask__subcmd__help__subcmd__tasks,attachments)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__attachments"
                ;;
            sealtask__subcmd__help__subcmd__tasks,complete)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__complete"
                ;;
            sealtask__subcmd__help__subcmd__tasks,create)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__create"
                ;;
            sealtask__subcmd__help__subcmd__tasks,delete)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__delete"
                ;;
            sealtask__subcmd__help__subcmd__tasks,edit)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__edit"
                ;;
            sealtask__subcmd__help__subcmd__tasks,get)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__get"
                ;;
            sealtask__subcmd__help__subcmd__tasks,list)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__tasks,move)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__move"
                ;;
            sealtask__subcmd__help__subcmd__tasks,reopen)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__reopen"
                ;;
            sealtask__subcmd__help__subcmd__tasks,resolve)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__resolve"
                ;;
            sealtask__subcmd__help__subcmd__tasks,task-references)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references"
                ;;
            sealtask__subcmd__help__subcmd__tasks,unarchive)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__unarchive"
                ;;
            sealtask__subcmd__help__subcmd__tasks,update)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__update"
                ;;
            sealtask__subcmd__help__subcmd__tasks,watch)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__watch"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__attachments,delete)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__delete"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__attachments,download)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__download"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__attachments,read)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__read"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__attachments,upload)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__upload"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references,quarantine)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references__subcmd__quarantine"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references,repair)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references__subcmd__repair"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references,status)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references__subcmd__status"
                ;;
            sealtask__subcmd__notes,create)
                cmd="sealtask__subcmd__notes__subcmd__create"
                ;;
            sealtask__subcmd__notes,delete)
                cmd="sealtask__subcmd__notes__subcmd__delete"
                ;;
            sealtask__subcmd__notes,edit)
                cmd="sealtask__subcmd__notes__subcmd__edit"
                ;;
            sealtask__subcmd__notes,get)
                cmd="sealtask__subcmd__notes__subcmd__get"
                ;;
            sealtask__subcmd__notes,help)
                cmd="sealtask__subcmd__notes__subcmd__help"
                ;;
            sealtask__subcmd__notes,list)
                cmd="sealtask__subcmd__notes__subcmd__list"
                ;;
            sealtask__subcmd__notes,update)
                cmd="sealtask__subcmd__notes__subcmd__update"
                ;;
            sealtask__subcmd__notes__subcmd__help,create)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__create"
                ;;
            sealtask__subcmd__notes__subcmd__help,delete)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__delete"
                ;;
            sealtask__subcmd__notes__subcmd__help,edit)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__edit"
                ;;
            sealtask__subcmd__notes__subcmd__help,get)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__get"
                ;;
            sealtask__subcmd__notes__subcmd__help,help)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__notes__subcmd__help,list)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__notes__subcmd__help,update)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__update"
                ;;
            sealtask__subcmd__pick,help)
                cmd="sealtask__subcmd__pick__subcmd__help"
                ;;
            sealtask__subcmd__pick,project)
                cmd="sealtask__subcmd__pick__subcmd__project"
                ;;
            sealtask__subcmd__pick,task)
                cmd="sealtask__subcmd__pick__subcmd__task"
                ;;
            sealtask__subcmd__pick__subcmd__help,help)
                cmd="sealtask__subcmd__pick__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__pick__subcmd__help,project)
                cmd="sealtask__subcmd__pick__subcmd__help__subcmd__project"
                ;;
            sealtask__subcmd__pick__subcmd__help,task)
                cmd="sealtask__subcmd__pick__subcmd__help__subcmd__task"
                ;;
            sealtask__subcmd__profile,help)
                cmd="sealtask__subcmd__profile__subcmd__help"
                ;;
            sealtask__subcmd__profile,list)
                cmd="sealtask__subcmd__profile__subcmd__list"
                ;;
            sealtask__subcmd__profile,use)
                cmd="sealtask__subcmd__profile__subcmd__use"
                ;;
            sealtask__subcmd__profile__subcmd__help,help)
                cmd="sealtask__subcmd__profile__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__profile__subcmd__help,list)
                cmd="sealtask__subcmd__profile__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__profile__subcmd__help,use)
                cmd="sealtask__subcmd__profile__subcmd__help__subcmd__use"
                ;;
            sealtask__subcmd__projects,archive)
                cmd="sealtask__subcmd__projects__subcmd__archive"
                ;;
            sealtask__subcmd__projects,audit)
                cmd="sealtask__subcmd__projects__subcmd__audit"
                ;;
            sealtask__subcmd__projects,clear)
                cmd="sealtask__subcmd__projects__subcmd__clear"
                ;;
            sealtask__subcmd__projects,current)
                cmd="sealtask__subcmd__projects__subcmd__current"
                ;;
            sealtask__subcmd__projects,get)
                cmd="sealtask__subcmd__projects__subcmd__get"
                ;;
            sealtask__subcmd__projects,help)
                cmd="sealtask__subcmd__projects__subcmd__help"
                ;;
            sealtask__subcmd__projects,list)
                cmd="sealtask__subcmd__projects__subcmd__list"
                ;;
            sealtask__subcmd__projects,sections)
                cmd="sealtask__subcmd__projects__subcmd__sections"
                ;;
            sealtask__subcmd__projects,unarchive)
                cmd="sealtask__subcmd__projects__subcmd__unarchive"
                ;;
            sealtask__subcmd__projects__subcmd__help,archive)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__archive"
                ;;
            sealtask__subcmd__projects__subcmd__help,audit)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__audit"
                ;;
            sealtask__subcmd__projects__subcmd__help,clear)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__clear"
                ;;
            sealtask__subcmd__projects__subcmd__help,current)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__current"
                ;;
            sealtask__subcmd__projects__subcmd__help,get)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__get"
                ;;
            sealtask__subcmd__projects__subcmd__help,help)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__projects__subcmd__help,list)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__projects__subcmd__help,sections)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__sections"
                ;;
            sealtask__subcmd__projects__subcmd__help,unarchive)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__unarchive"
                ;;
            sealtask__subcmd__projects__subcmd__help__subcmd__sections,list)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__sections__subcmd__list"
                ;;
            sealtask__subcmd__projects__subcmd__sections,help)
                cmd="sealtask__subcmd__projects__subcmd__sections__subcmd__help"
                ;;
            sealtask__subcmd__projects__subcmd__sections,list)
                cmd="sealtask__subcmd__projects__subcmd__sections__subcmd__list"
                ;;
            sealtask__subcmd__projects__subcmd__sections__subcmd__help,help)
                cmd="sealtask__subcmd__projects__subcmd__sections__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__projects__subcmd__sections__subcmd__help,list)
                cmd="sealtask__subcmd__projects__subcmd__sections__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__tasks,archive)
                cmd="sealtask__subcmd__tasks__subcmd__archive"
                ;;
            sealtask__subcmd__tasks,attachments)
                cmd="sealtask__subcmd__tasks__subcmd__attachments"
                ;;
            sealtask__subcmd__tasks,complete)
                cmd="sealtask__subcmd__tasks__subcmd__complete"
                ;;
            sealtask__subcmd__tasks,create)
                cmd="sealtask__subcmd__tasks__subcmd__create"
                ;;
            sealtask__subcmd__tasks,delete)
                cmd="sealtask__subcmd__tasks__subcmd__delete"
                ;;
            sealtask__subcmd__tasks,edit)
                cmd="sealtask__subcmd__tasks__subcmd__edit"
                ;;
            sealtask__subcmd__tasks,get)
                cmd="sealtask__subcmd__tasks__subcmd__get"
                ;;
            sealtask__subcmd__tasks,help)
                cmd="sealtask__subcmd__tasks__subcmd__help"
                ;;
            sealtask__subcmd__tasks,list)
                cmd="sealtask__subcmd__tasks__subcmd__list"
                ;;
            sealtask__subcmd__tasks,move)
                cmd="sealtask__subcmd__tasks__subcmd__move"
                ;;
            sealtask__subcmd__tasks,reopen)
                cmd="sealtask__subcmd__tasks__subcmd__reopen"
                ;;
            sealtask__subcmd__tasks,resolve)
                cmd="sealtask__subcmd__tasks__subcmd__resolve"
                ;;
            sealtask__subcmd__tasks,task-references)
                cmd="sealtask__subcmd__tasks__subcmd__task__subcmd__references"
                ;;
            sealtask__subcmd__tasks,unarchive)
                cmd="sealtask__subcmd__tasks__subcmd__unarchive"
                ;;
            sealtask__subcmd__tasks,update)
                cmd="sealtask__subcmd__tasks__subcmd__update"
                ;;
            sealtask__subcmd__tasks,watch)
                cmd="sealtask__subcmd__tasks__subcmd__watch"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments,delete)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__delete"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments,download)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__download"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments,help)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments,read)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__read"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments,upload)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__upload"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments__subcmd__help,delete)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__delete"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments__subcmd__help,download)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__download"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments__subcmd__help,help)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments__subcmd__help,read)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__read"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments__subcmd__help,upload)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__upload"
                ;;
            sealtask__subcmd__tasks__subcmd__help,archive)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__archive"
                ;;
            sealtask__subcmd__tasks__subcmd__help,attachments)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__attachments"
                ;;
            sealtask__subcmd__tasks__subcmd__help,complete)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__complete"
                ;;
            sealtask__subcmd__tasks__subcmd__help,create)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__create"
                ;;
            sealtask__subcmd__tasks__subcmd__help,delete)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__delete"
                ;;
            sealtask__subcmd__tasks__subcmd__help,edit)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__edit"
                ;;
            sealtask__subcmd__tasks__subcmd__help,get)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__get"
                ;;
            sealtask__subcmd__tasks__subcmd__help,help)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__tasks__subcmd__help,list)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__tasks__subcmd__help,move)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__move"
                ;;
            sealtask__subcmd__tasks__subcmd__help,reopen)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__reopen"
                ;;
            sealtask__subcmd__tasks__subcmd__help,resolve)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__resolve"
                ;;
            sealtask__subcmd__tasks__subcmd__help,task-references)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references"
                ;;
            sealtask__subcmd__tasks__subcmd__help,unarchive)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__unarchive"
                ;;
            sealtask__subcmd__tasks__subcmd__help,update)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__update"
                ;;
            sealtask__subcmd__tasks__subcmd__help,watch)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__watch"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__attachments,delete)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__delete"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__attachments,download)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__download"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__attachments,read)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__read"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__attachments,upload)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__upload"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references,quarantine)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references__subcmd__quarantine"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references,repair)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references__subcmd__repair"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references,status)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references__subcmd__status"
                ;;
            sealtask__subcmd__tasks__subcmd__task__subcmd__references,help)
                cmd="sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help"
                ;;
            sealtask__subcmd__tasks__subcmd__task__subcmd__references,quarantine)
                cmd="sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__quarantine"
                ;;
            sealtask__subcmd__tasks__subcmd__task__subcmd__references,repair)
                cmd="sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__repair"
                ;;
            sealtask__subcmd__tasks__subcmd__task__subcmd__references,status)
                cmd="sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__status"
                ;;
            sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help,help)
                cmd="sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help,quarantine)
                cmd="sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help__subcmd__quarantine"
                ;;
            sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help,repair)
                cmd="sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help__subcmd__repair"
                ;;
            sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help,status)
                cmd="sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help__subcmd__status"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        sealtask)
            opts="-q -v -h -V --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --serve-unlock-daemon --help --version completion man info schema auth me pick projects lists tasks agents stats activity browse cache batch doctor config profile inspect comments notes help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --serve-unlock-daemon)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__activity)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help follow help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__activity__subcmd__follow)
            opts="-q -v -h --interval --since --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --interval)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --since)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__activity__subcmd__help)
            opts="follow help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__activity__subcmd__help__subcmd__follow)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__activity__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help register approve list revoke assign unassign help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__approve)
            opts="-q -v -h --enrollment-code-file --fingerprint --handle --display-name --instructions-file --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --enrollment-code-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fingerprint)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --handle)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --display-name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --instructions-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__assign)
            opts="-q -v -h --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__help)
            opts="register approve list revoke assign unassign help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__help__subcmd__approve)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__help__subcmd__assign)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__help__subcmd__register)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__help__subcmd__revoke)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__help__subcmd__unassign)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__list)
            opts="-q -v -h --local --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__register)
            opts="-q -v -h --proposed-handle --project --work-list-id --repository --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --proposed-handle)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --repository)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__revoke)
            opts="-q -v -h --reason --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --reason)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__agents__subcmd__unassign)
            opts="-q -v -h --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help login unlock lock keychain logout status help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help)
            opts="login unlock lock keychain logout status help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__keychain)
            opts="store clear"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__keychain__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__keychain__subcmd__store)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__lock)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__login)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__logout)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__unlock)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help store clear help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__clear)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__help)
            opts="store clear help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__store)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__store)
            opts="-q -v -h --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__lock)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__login)
            opts="-q -v -h --email --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --email)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__logout)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__status)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__unlock)
            opts="-q -v -h --ttl --ttl-seconds --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ttl)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ttl-seconds)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__batch)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help run help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__batch__subcmd__help)
            opts="run help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__batch__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__batch__subcmd__help__subcmd__run)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__batch__subcmd__run)
            opts="-q -v -h --input --jobs --continue-on-error --checkpoint --resume --dry-run --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --input)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --jobs)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --checkpoint)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__browse)
            opts="-q -v -h --include-completed --include-archived --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__cache)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help status verify clear help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__cache__subcmd__clear)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__cache__subcmd__help)
            opts="status verify clear help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__cache__subcmd__help__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__cache__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__cache__subcmd__help__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__cache__subcmd__help__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__cache__subcmd__status)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__cache__subcmd__verify)
            opts="-q -v -h --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help list create update delete help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__create)
            opts="-q -v -h --task-id --project --work-list-id --body --body-file --input-file --input-stdin --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__delete)
            opts="-q -v -h --task-id --project --work-list-id --comment-id --input-file --input-stdin --password-stdin --yes --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --comment-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help)
            opts="list create update delete help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__list)
            opts="-q -v -h --project --work-list-id --task-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__update)
            opts="-q -v -h --task-id --project --work-list-id --comment-id --body --input-file --input-stdin --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --comment-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__completion)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help bash zsh fish powershell"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__config)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help show help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__config__subcmd__help)
            opts="show help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__config__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__config__subcmd__help__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__config__subcmd__show)
            opts="-q -v -h --resolved --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__doctor)
            opts="-q -v -h --strict --include-keychain --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help)
            opts="completion man info schema auth me pick projects tasks agents stats activity browse cache batch doctor config profile inspect comments notes help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__activity)
            opts="follow"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__activity__subcmd__follow)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__agents)
            opts="register approve list revoke assign unassign"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__agents__subcmd__approve)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__agents__subcmd__assign)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__agents__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__agents__subcmd__register)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__agents__subcmd__revoke)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__agents__subcmd__unassign)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth)
            opts="login unlock lock keychain logout status"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__keychain)
            opts="store clear"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__keychain__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__keychain__subcmd__store)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__lock)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__login)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__logout)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__unlock)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__batch)
            opts="run"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__batch__subcmd__run)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__browse)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__cache)
            opts="status verify clear"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__cache__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__cache__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__cache__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__comments)
            opts="list create update delete"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__comments__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__comments__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__comments__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__comments__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__completion)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__config)
            opts="show"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__config__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__doctor)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__info)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__inspect)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__man)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__me)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes)
            opts="list get create edit update delete"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__edit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__pick)
            opts="project task"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__pick__subcmd__project)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__pick__subcmd__task)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__profile)
            opts="list use"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__profile__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__profile__subcmd__use)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects)
            opts="list get archive unarchive current clear sections audit"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__archive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__audit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__current)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__sections)
            opts="list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__sections__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__unarchive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__schema)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks)
            opts="list get resolve task-references watch create edit update move complete reopen archive unarchive delete attachments"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__archive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__attachments)
            opts="upload delete read download"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__download)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__read)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__upload)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__complete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__edit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__move)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__reopen)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__resolve)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references)
            opts="status repair quarantine"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references__subcmd__quarantine)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references__subcmd__repair)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__task__subcmd__references__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__unarchive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__watch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__info)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__inspect)
            opts="-q -v -h --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__man)
            opts="-q -v -h --output-dir --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --output-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__me)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help list get create edit update delete help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__create)
            opts="-q -v -h --project --work-list-id --title --body --private --idempotency-key --input-file --input-stdin --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --title)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --idempotency-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__delete)
            opts="-q -v -h --note-id --project --work-list-id --input-file --input-stdin --password-stdin --yes --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --note-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__edit)
            opts="-q -v -h --note-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --note-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__get)
            opts="-q -v -h --note-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --note-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help)
            opts="list get create edit update delete help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__edit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__list)
            opts="-q -v -h --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__update)
            opts="-q -v -h --note-id --project --work-list-id --title --body --input-file --input-stdin --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --note-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --title)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__pick)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help project task help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__pick__subcmd__help)
            opts="project task help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__pick__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__pick__subcmd__help__subcmd__project)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__pick__subcmd__help__subcmd__task)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__pick__subcmd__project)
            opts="-q -v -h --include-archived --scope --print-selector --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --scope)
                    COMPREPLY=($(compgen -W "local global" -- "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__pick__subcmd__task)
            opts="-q -v -h --project --work-list-id --include-completed --include-archived --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help list use help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__help)
            opts="list use help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__help__subcmd__use)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__list)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__use)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects)
            opts="-q -v -h --verbose --include-archived --password-stdin --raw --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help list get archive unarchive current clear sections audit help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__archive)
            opts="-q -v -h --password-stdin --raw --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__audit)
            opts="-q -v -h --work-list-id --cursor --limit --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cursor)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__clear)
            opts="-q -v -h --scope --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --scope)
                    COMPREPLY=($(compgen -W "local global" -- "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__current)
            opts="-q -v -h --scope --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --scope)
                    COMPREPLY=($(compgen -W "local global" -- "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__get)
            opts="-q -v -h --password-stdin --raw --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help)
            opts="list get archive unarchive current clear sections audit help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__archive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__audit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__current)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__sections)
            opts="list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__sections__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__unarchive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__list)
            opts="-q -v -h --details --include-archived --password-stdin --raw --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__sections)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__sections__subcmd__help)
            opts="list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__sections__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__sections__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__sections__subcmd__list)
            opts="-q -v -h --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__unarchive)
            opts="-q -v -h --password-stdin --raw --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__schema)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__stats)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help list get resolve task-references watch create edit update move complete reopen archive unarchive delete attachments help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__archive)
            opts="-q -v -h --task-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help upload delete read download help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__delete)
            opts="-q -v -h --task-id --project --work-list-id --attachment-id --password-stdin --yes --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --attachment-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__download)
            opts="-q -v -h --task-id --project --work-list-id --attachment-id --output --force --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --attachment-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help)
            opts="upload delete read download help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__download)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__read)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__upload)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__read)
            opts="-q -v -h --task-id --project --work-list-id --attachment-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --attachment-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__upload)
            opts="-q -v -h --task-id --project --work-list-id --file --file-name --content-type --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file-name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --content-type)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__complete)
            opts="-q -v -h --task-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__create)
            opts="-q -v -h --project --work-list-id --title --body --body-file --edit --priority --due-at --due --start-at --section-id --section --idempotency-key --input-file --input-stdin --dry-run --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --title)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --priority)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --due-at)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --due)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --start-at)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --idempotency-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__delete)
            opts="-q -v -h --task-id --project --work-list-id --input-file --input-stdin --password-stdin --yes --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__edit)
            opts="-q -v -h --task-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__get)
            opts="-q -v -h --task-id --project --work-list-id --password-stdin --raw --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help)
            opts="list get resolve task-references watch create edit update move complete reopen archive unarchive delete attachments help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__archive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__attachments)
            opts="upload delete read download"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__download)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__read)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__upload)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__complete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__edit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__move)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__reopen)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__resolve)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references)
            opts="status repair quarantine"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references__subcmd__quarantine)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references__subcmd__repair)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__task__subcmd__references__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__unarchive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__watch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__list)
            opts="-q -v -h --project --work-list-id --include-completed --include-archived --all --columns --sort --field --web-url --password-stdin --raw --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --columns)
                    COMPREPLY=($(compgen -W "reference id title project project-id priority due status comments created updated" -- "${cur}"))
                    return 0
                    ;;
                --sort)
                    COMPREPLY=($(compgen -W "reference id title project priority due status created updated" -- "${cur}"))
                    return 0
                    ;;
                --field)
                    COMPREPLY=($(compgen -W "reference id title url" -- "${cur}"))
                    return 0
                    ;;
                --web-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__move)
            opts="-q -v -h --task-id --project --work-list-id --section-id --section --insert-before-task-id --before --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --insert-before-task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --before)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__reopen)
            opts="-q -v -h --task-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__resolve)
            opts="-q -v -h --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__task__subcmd__references)
            opts="-q -v -h --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help status repair quarantine help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help)
            opts="status repair quarantine help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help__subcmd__quarantine)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help__subcmd__repair)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__help__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__quarantine)
            opts="-q -v -h --work-list-id --scheme-revision-id --confirm --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --scheme-revision-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__repair)
            opts="-q -v -h --work-list-id --prefix --minimum-digits --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --prefix)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --minimum-digits)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__task__subcmd__references__subcmd__status)
            opts="-q -v -h --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__unarchive)
            opts="-q -v -h --task-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__update)
            opts="-q -v -h --task-id --project --work-list-id --title --body --body-file --clear-body --priority --clear-priority --due-at --due --clear-due-at --start-at --clear-start-at --section-id --section --clear-section --input-file --input-stdin --dry-run --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --title)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --priority)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --due-at)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --due)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --start-at)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__watch)
            opts="-q -v -h --project --work-list-id --include-completed --include-archived --password-stdin --api-url --storage-origin --json --format --color --pager --no-pager --progress --quiet --non-interactive --debug --connect-timeout --read-timeout --request-timeout --retry --offline --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty jsonl" -- "${cur}"))
                    return 0
                    ;;
                --color)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --pager)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --progress)
                    COMPREPLY=($(compgen -W "auto always never" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --retry)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _sealtask -o nosort -o bashdefault -o default sealtask
else
    complete -F _sealtask -o bashdefault -o default sealtask
fi
